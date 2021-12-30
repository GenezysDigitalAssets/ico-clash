use solana_program::{
    decode_error::DecodeError, entrypoint::ProgramResult, msg, program_error::ProgramError,
};
use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, Error, PartialEq)]
pub enum ICOError {
    // Parse instructions and data
    #[error("Invalid instruction data: No data was passed to program")]
    InvalidInstructionDataEmpty,

    #[error("Invalid program instruction")]
    InvalidProgramInstruction,

    // General errors
    #[error("Invalid Clash token ID")]
    InvalidClashTokenId,

    #[error("Program account PDA does not match the expected PDA")]
    InvalidAddressProgramPDA,

    // Initialize ICO
    #[error("Program PDA account already exists! Terminate the current ICO and try again")]
    AlreadyCreatedPDAAccount,

    #[error("Initializer is not the token mint authority")]
    InitializerNotMintAuthority,

    #[error("Unexpected address for initializer associated token account")]
    InvalidInitializerATAAddress,

    // Exchange Clash tokens
    #[error("Cannot transfer SOL from/to the same account")]
    CannotTransferSameAccount,

    #[error("Cannot transfer CLASH from/to the same associated account")]
    CannotTransferSameAssociatedAccount,

    #[error("Invalid source associated account owner: must be owned by the source SOL account")]
    InvalidSourceAssociatedAccountOwner,

    #[error("Invalid program associated account owner: must be owner by the program PDA")]
    InvalidProgramAssociatedPDAOwner,

    #[error("Invalid Clash token destination account address")]
    InvalidClashTokenDestinationWallet,

    #[error("Invalid offer because its value in USD is bellow the limit")]
    InvalidOfferTooFew,

    #[error("Invalid offer because its value in USD is above the limit")]
    InvalidOfferTooMuch,

    #[error("Invalid Clash token count: more SOL may be required")]
    InvalidClashTokenAmount,

    #[error("Not enough Clash tokens available to exchange")]
    InsuficientClashToken,

    // Execute Clash Payment
    #[error("Invalid Clash trusted payment authority")]
    InvalidClashTrustedAuthority,

    // Terminate ICO
    #[error("There is not an initialized ICO to terminate")]
    InvalidTerminateUninitializedICO,

    #[error("Incorrect initializer account")]
    InitializerAccountMismatch,

    #[error("Incorrect initializer associated token account")]
    InitializerAssociatedAccountMismatch,
}

impl From<ICOError> for ProgramError {
    fn from(e: ICOError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<E> DecodeError<E> for ICOError {
    fn type_of() -> &'static str {
        "ICOError"
    }
}

pub fn ico_err(err: ICOError) -> ProgramResult {
    let err_code: u32 = err as u32;
    msg!("[ICOError #{}] Reason: '{}'", err_code, err);
    Err(err.into())
}
