use crate::error::TokenStarterError;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, BurnChecked, Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(
        mut,
        constraint = token_account.mint == mint.key() @ TokenStarterError::InvalidMint,
        constraint = token_account.owner == authority.key() @ TokenStarterError::InvalidAuthority,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
    require!(amount > 0, TokenStarterError::InvalidAmount);

    let cpi_accounts = BurnChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);

    token_interface::burn_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    Ok(())
}
