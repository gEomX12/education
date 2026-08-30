use anchor_lang::prelude::*;

#[error_code]
pub enum TokenStarterError {
    #[msg("Amount must be greater than zero")]
    AmountMustBePositive,
    #[msg("Source and destination token accounts must be different")]
    SourceEqualsDestination,
    #[msg("Mint does not match the token account")]
    InvalidMint,
    #[msg("Authority does not own the token account")]
    InvalidAuthority,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
}

pub use TokenStarterError as CustomError;
