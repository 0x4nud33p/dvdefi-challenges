use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::errors::AmmError;
use crate::AmmState;
use crate::constants::{AMMSEED, AMM_LP_MINT};
use constant_product_curve::ConstantProduct;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        seeds = [AMMSEED, mint_x.key().as_ref(), mint_y.key().as_ref(), &state.seed.to_le_bytes()],
        bump = state.bump,
        has_one = mint_x,
        has_one = mint_y,
    )]
    pub state: Account<'info, AmmState>,

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
        associated_token::mint = state.mint_x,
        associated_token::authority = user,
    )]
    pub user_ata_x: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = state.mint_y,
        associated_token::authority = user,
    )]
    pub user_ata_y: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [AMM_LP_MINT, state.key().as_ref()],
        bump = state.lp_bump,
    )]
    pub mint_lp: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_ata_lp: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn handler(&mut self, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        require!(self.state.is_locked == false, AmmError::AmmLocked);
        require!(amount != 0, AmmError::InvalidAmount);

        // Calculate required amounts of X and Y based on current reserves and total LP supply
        let (x, y) = match self.mint_lp.supply == 0
            && self.vault_x.amount == 0
            && self.vault_y.amount == 0
        {
            true => (max_x, max_y),
            false => {
                // Using constant product formula to determine required amounts
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_x.amount,
                    self.vault_y.amount,
                    self.mint_lp.supply,
                    amount,
                    6,
                )
                .unwrap();
                (amounts.x, amounts.y)
            }
        };

        require!(x <= max_x && y <= max_y, AmmError::SlippageExceeded);
        self.deposit_tokens(true, x)?;
        self.deposit_tokens(false, y)?;
        self.mint_lp_tokens(amount)
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

    pub fn mint_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_ata_lp.to_account_info(),
            authority: self.state.to_account_info(),
        };

        let seeds = &[
            AMMSEED,
            self.state.mint_x.as_ref(),
            self.state.mint_y.as_ref(),
            &self.state.seed.to_le_bytes(),
            &[self.state.bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        mint_to(ctx, amount)
    }
}
