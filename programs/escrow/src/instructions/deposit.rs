use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::ID as TOKEN_2022_PROGRAM_ID,
    token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::EscrowError;
use crate::state::{EscrowState, EscrowStatus, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(deal_id: u64)]
pub struct Deposit<'info> {
    pub sender: Signer<'info>,
    #[account(
        mut,
        seeds = [ESCROW_SEED, sender.key().as_ref(), deal_id.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = sender @ EscrowError::InvalidSender,
        has_one = mint @ EscrowError::InvalidMint,
        constraint = escrow.status == EscrowStatus::Created @ EscrowError::AlreadyProcessed,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = sender,
        token::token_program = token_program,
    )]
    pub sender_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        constraint = token_program.key() == TOKEN_2022_PROGRAM_ID @ EscrowError::InvalidTokenProgram
    )]
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Deposit>, _deal_id: u64) -> Result<()> {
    let amount = ctx.accounts.escrow.amount;
    require!(amount > 0, EscrowError::InvalidAmount);

    let decimals = ctx.accounts.mint.decimals;
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.sender_token.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
    token_interface::transfer_checked(cpi_ctx, amount, decimals)?;

    ctx.accounts.escrow.status = ctx.accounts.escrow.status.fund()?;

    Ok(())
}
