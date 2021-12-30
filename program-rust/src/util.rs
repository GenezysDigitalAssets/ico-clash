use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use spl_token::state::Account as TokenAccount;

// Helper function to avoid repeating code for account validation
pub fn validate_account<'a>(
    acc_info: &AccountInfo,
    signer: bool,
    writable: bool,
    initialized: bool,
) -> ProgramResult {
    if signer && !acc_info.is_signer {
        msg!(format!(
            "Invalid account info(`{}`): Missing signature!",
            acc_info.key
        )
        .as_str());
        Err(ProgramError::MissingRequiredSignature)?;
    }

    if writable && !acc_info.is_writable {
        msg!(format!(
            "Invalid account info(`{}`): Missing writeable status!",
            acc_info.key
        )
        .as_str());
        Err(ProgramError::InvalidArgument)?;
    }

    if initialized && !acc_info.lamports() == 0 {
        msg!(format!(
            "Invalid account info(`{}`): Unfunded account! Balance is 0 lamports.",
            acc_info.key
        )
        .as_str());
        Err(ProgramError::UninitializedAccount)?;
    }

    Ok(())
}

// Helper function to avoid repeating code for token account validation
pub fn validate_token_account(
    acc_info: &TokenAccount,
    owner: &Pubkey,
    mint: &Pubkey,
) -> ProgramResult {
    if &acc_info.owner != owner {
        msg!(format!(
            "Invalid associated token account: Owner mismatch!\nExpected Owner: {}\nAccount Owner: {}",
            owner, acc_info.owner
        )
        .as_str());

        Err(ProgramError::IllegalOwner)?;
    }

    if &acc_info.mint != mint {
        msg!(format!(
            "Invalid associated token account: Mint mismatch!\nExpected Mint: {}\nAccount Mint: {}",
            mint, acc_info.mint
        )
        .as_str());

        Err(ProgramError::InvalidArgument)?;
    }

    Ok(())
}
