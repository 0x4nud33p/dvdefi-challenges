use crate::constants::{AMMSEED, AMM_LP_MINT};
use crate::errors::AmmError;
use crate::AmmState;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        seeds = [AMMSEED, mint_x.key().as_ref(), mint_y.key().as_ref(), &state.seed.to_le_bytes()],
        has_one = mint_x,
        has_one = mint_y,
        bump = state.bump,
    )]
    pub state: Account<'info, AmmState>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = state,
    )]
    pub vault_x: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = state,
    )]
    pub vault_y: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_ata_x: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_ata_y: Account<'info, TokenAccount>,
    #[account(
        seeds = [AMM_LP_MINT, state.key().as_ref()],
        bump = state.lp_bump,
    )]
    pub mint_lp: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Swap<'info> {
    pub fn handler(&mut self, is_x: bool, amount_in: u64, min_amount_out: u64) -> Result<()> {
        require!(!self.state.is_locked, AmmError::AmmLocked);
        require!(amount_in != 0, AmmError::InvalidAmount);

        // Initialize the constant product curve with current vault balances and LP supply
        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.state.fee,
            None,
        )
        .map_err(AmmError::from)?;

        // get the liquidity pair based on swap direction
        let pair = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };

        // Perform the swap calculation
        let res = curve
            .swap(pair, amount_in, min_amount_out)
            .map_err(AmmError::from)?;

        require!(
            res.deposit != 0 && res.withdraw != 0,
            AmmError::InvalidAmount
        );

        self.deposit_tokens(is_x, res.deposit)?;
        self.withdraw_tokens(is_x, res.withdraw)?;

        Ok(())
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.user_ata_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.user_ata_y.to_account_info(),
                self.vault_y.to_account_info(),
            ),
        };

        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.user.to_account_info(),
        };

        let ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(ctx, amount)
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_ata_x.to_account_info(),
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_ata_y.to_account_info(),
            ),
        };

        let seeds = &[
            AMMSEED,
            self.state.mint_x.as_ref(),
            self.state.mint_y.as_ref(),
            &self.state.seed.to_le_bytes(),
            &[self.state.bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.state.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );

        transfer(ctx, amount)
    }
}
