use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::ID as TOKEN_2022_PROGRAM_ID,
    token_interface::{self, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::error::EscrowError;
use crate::state::{EscrowState, EscrowStatus, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(deal_id: u64)]
pub struct Release<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: token destination owner; must match escrow.receiver, does not sign.
    pub receiver: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [ESCROW_SEED, sender.key().as_ref(), deal_id.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = sender @ EscrowError::InvalidSender,
        has_one = receiver @ EscrowError::InvalidReceiver,
        has_one = mint @ EscrowError::InvalidMint,
        constraint = escrow.status == EscrowStatus::Funded @ EscrowError::AlreadyProcessed,
        close = sender,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = escrow,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = receiver,
        token::token_program = token_program,
    )]
    pub receiver_token: InterfaceAccount<'info, TokenAccount>,
    #[account(
        constraint = token_program.key() == TOKEN_2022_PROGRAM_ID @ EscrowError::InvalidTokenProgram
    )]
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Release>, deal_id: u64) -> Result<()> {
    let amount = ctx.accounts.escrow.amount;
    let decimals = ctx.accounts.mint.decimals;
    let bump = ctx.accounts.escrow.bump;
    let sender_key = ctx.accounts.sender.key();
    let deal_id_bytes = deal_id.to_le_bytes();
    let signer_seeds: &[&[u8]] = &[
        ESCROW_SEED,
        sender_key.as_ref(),
        deal_id_bytes.as_ref(),
        &[bump],
    ];
    let signer = &[signer_seeds];

    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TransferChecked {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.receiver_token.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer,
        ),
        amount,
        decimals,
    )?;

    token_interface::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.sender.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer,
    ))?;

    ctx.accounts.escrow.status = ctx.accounts.escrow.status.release()?;

    Ok(())
}
