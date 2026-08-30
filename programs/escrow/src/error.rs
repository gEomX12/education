use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Sender and receiver must be different")]
    SenderEqualsReceiver,
    #[msg("Escrow is not in the required status")]
    InvalidStatus,
    #[msg("Signer is not the sender")]
    InvalidSender,
    #[msg("Signer is not the receiver")]
    InvalidReceiver,
    #[msg("Mint does not match the escrow")]
    InvalidMint,
    #[msg("Token program must be Token-2022")]
    InvalidTokenProgram,
    #[msg("This instruction has already been executed")]
    AlreadyProcessed,
}
