use anchor_lang::prelude::*;

use crate::error::EscrowError;

pub const ESCROW_SEED: &[u8] = b"escrow";

#[derive(InitSpace, AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscrowStatus {
    Created,
    Funded,
    Released,
    Cancelled,
}

impl EscrowStatus {
    pub fn fund(self) -> Result<Self> {
        match self {
            Self::Created => Ok(Self::Funded),
            Self::Funded | Self::Released | Self::Cancelled => {
                err!(EscrowError::AlreadyProcessed)
            }
        }
    }

    pub fn release(self) -> Result<Self> {
        match self {
            Self::Funded => Ok(Self::Released),
            Self::Released | Self::Cancelled => err!(EscrowError::AlreadyProcessed),
            Self::Created => err!(EscrowError::InvalidStatus),
        }
    }

    pub fn cancel(self) -> Result<Self> {
        match self {
            Self::Created | Self::Funded => Ok(Self::Cancelled),
            Self::Released | Self::Cancelled => err!(EscrowError::AlreadyProcessed),
        }
    }
}

#[account]
#[derive(InitSpace)]
pub struct EscrowState {
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub deal_id: u64,
    pub bump: u8,
    pub status: EscrowStatus,
}
