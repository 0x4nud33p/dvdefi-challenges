use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    associated_token::AssociatedToken,
};

use crate::AmmState;
use crate::constants::{AMMSEED, AMM_LP_MINT};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init, 
        payer = user, 
        space = AmmState::LEN + 8,
        seeds = [AMMSEED, mint_x.key().as_ref(), mint_y.key().as_ref(), &seed.to_le_bytes()],
        bump
    )]
    pub state: Account<'info, AmmState>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        init,
        payer = user,
        mint::decimals = 6,
        mint::authority = state, // state will be the mint authority cause it needs to mint/burn LP tokens no one else should
        seeds = [AMM_LP_MINT, state.key().as_ref()],
        bump
    )]
    pub mint_lp: Account<'info, Mint>, // LP Token Mint proof for deposit/withdraw
    #[account(
        init,
        payer = user,
        associated_token::mint = mint_x,
        associated_token::authority = state,
    )]
    pub vault_x: Account<'info, TokenAccount>, // only token 2022 accounts are using
    #[account(
        init,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = state,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}


impl<'info> Initialize<'info> {
    pub fn handler(&mut self, seed: u64, fee: u16, bumps: &InitializeBumps) -> Result<()> {
        self.state.set_inner(AmmState {
            seed,
            is_locked: false,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            mint_lp: self.mint_lp.key(),
            fee,
            bump: bumps.state,
            lp_bump: bumps.mint_lp,
        });

        Ok(())
    }
}