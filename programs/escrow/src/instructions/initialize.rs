use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::ID as TOKEN_2022_PROGRAM_ID,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::error::EscrowError;
use crate::state::{EscrowState, EscrowStatus, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(deal_id: u64, amount: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: counterparty pubkey stored on-chain; must differ from sender.
    pub receiver: UncheckedAccount<'info>,
    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = sender,
        space = 8 + EscrowState::INIT_SPACE,
        seeds = [ESCROW_SEED, sender.key().as_ref(), deal_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(
        init,
        payer = sender,
        associated_token::mint = mint,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        constraint = token_program.key() == TOKEN_2022_PROGRAM_ID @ EscrowError::InvalidTokenProgram
    )]
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, deal_id: u64, amount: u64) -> Result<()> {
    require!(amount > 0, EscrowError::InvalidAmount);
    require!(
        ctx.accounts.sender.key() != ctx.accounts.receiver.key(),
        EscrowError::SenderEqualsReceiver
    );

    let escrow = &mut ctx.accounts.escrow;
    escrow.sender = ctx.accounts.sender.key();
    escrow.receiver = ctx.accounts.receiver.key();
    escrow.mint = ctx.accounts.mint.key();
    escrow.amount = amount;
    escrow.deal_id = deal_id;
    escrow.bump = ctx.bumps.escrow;
    escrow.status = EscrowStatus::Created;

    Ok(())
}
