use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

declare_id!("FNz31K54MxJkxd2dUXuHgCs2KyjW2BtDZ8WkcnT6ExuR");

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, deal_id: u64, amount: u64) -> Result<()> {
        instructions::initialize::handler(ctx, deal_id, amount)
    }

    pub fn deposit(ctx: Context<Deposit>, deal_id: u64) -> Result<()> {
        instructions::deposit::handler(ctx, deal_id)
    }

    pub fn release(ctx: Context<Release>, deal_id: u64) -> Result<()> {
        instructions::release::handler(ctx, deal_id)
    }

    pub fn cancel(ctx: Context<Cancel>, deal_id: u64) -> Result<()> {
        instructions::cancel::handler(ctx, deal_id)
    }
}
