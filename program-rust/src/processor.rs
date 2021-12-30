use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::transfer,
    sysvar::Sysvar,
};

use spl_token::state::{Account as TokenAccount, Mint};

use crate::error::{ico_err, ICOError};

use crate::config::{
    CLASH_PAYMENT_AUTHORITY, CLASH_SOL_WALLET, CLASH_TOKEN_ID, CLASH_USD, MAX_USD_PRICE,
    MIN_USD_PRICE, PROGRAM_PDA_SEED1, PROGRAM_PDA_SEED2, SOL_USD,
};

use crate::state::{ClashTokenExchangeData, ClashTokenPaymentData, ICOProgramData};

use crate::instruction::ProgramInstruction;

use crate::util::{validate_account, validate_token_account};

use borsh::{BorshDeserialize, BorshSerialize};

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = ProgramInstruction::unpack(instruction_data)?;

        match instruction {
            ProgramInstruction::InitializeICO => {
                msg!("Instruction: Initialize Clash ICO");
                Self::initialize_ico(program_id, accounts)
            }
            ProgramInstruction::ExchangeClashToken { data } => {
                msg!("Instruction: Exchange Clash Token");
                Self::exchange_clash_token(program_id, accounts, &data)
            }
            ProgramInstruction::ExecuteClashPayment { data } => {
                msg!("Instruction: Execute Clash Payment");
                Self::execute_clash_payment(program_id, accounts, &data)
            }
            ProgramInstruction::TerminateICO => {
                msg!("Instruction: Terminate Clash ICO");
                Self::terminate_ico(program_id, accounts)
            }
            ProgramInstruction::InvalidInstruction => {
                msg!("Invalid instruction");
                Err(ProgramError::InvalidInstructionData)?
            }
        }?;

        Ok(())
    }

    pub fn initialize_ico(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        msg!("Initializing Clash ICO accounts and data");

        // Get accounts
        let accounts_iter = &mut accounts.iter();

        let initializer_account = next_account_info(accounts_iter)?;
        let initializer_token_account = next_account_info(accounts_iter)?;

        let clash_token_account = next_account_info(accounts_iter)?;

        let program_pda_account = next_account_info(accounts_iter)?;
        let program_token_account = next_account_info(accounts_iter)?;

        let system_program_account = next_account_info(accounts_iter)?;
        let token_program_account = next_account_info(accounts_iter)?;
        let associated_token_account_program = next_account_info(accounts_iter)?;
        let sysvar_rent_program_account = next_account_info(accounts_iter)?;

        validate_account(initializer_account, true, false, true)?;
        validate_account(initializer_token_account, false, false, true)?;

        validate_account(clash_token_account, false, true, true)?;

        validate_account(program_pda_account, false, true, false)?;
        validate_account(program_token_account, false, true, false)?;

        let initializer_associated_token_account =
            TokenAccount::unpack_unchecked(&initializer_token_account.data.borrow())?;

        validate_token_account(
            &initializer_associated_token_account,
            initializer_account.key,
            &CLASH_TOKEN_ID,
        )?;

        let (program_pda, bump_seed) =
            Pubkey::find_program_address(&[PROGRAM_PDA_SEED1, PROGRAM_PDA_SEED2], program_id);

        let program_signature = &[&PROGRAM_PDA_SEED1, &PROGRAM_PDA_SEED2, &[bump_seed][..]];

        if program_pda_account.key != &program_pda {
            ico_err(ICOError::InvalidAddressProgramPDA)?;
        };

        if program_pda_account.lamports() != 0 {
            let ico_data = ICOProgramData::try_from_slice(&program_pda_account.data.borrow())?;
            msg!("ICO was already initialized by `{}`", ico_data.initializer);

            ico_err(ICOError::AlreadyCreatedPDAAccount)?;
        }

        let clash_mint_data = Mint::unpack_unchecked(&clash_token_account.data.borrow())?;

        if !clash_mint_data
            .mint_authority
            .contains(initializer_account.key)
        {
            ico_err(ICOError::InitializerNotMintAuthority)?;
        }

        msg!(format!(
            "Creating ICO data account(PDA): `{}`\nInitializer: `{}`",
            program_pda_account.key, initializer_account.key
        )
        .as_str());

        let data_size = std::mem::size_of::<ICOProgramData>();

        // Calculate minimum rent to make this account rent-exempt
        // Lamports will be transferred back to owner account once this account is closed
        let rent_sysvar = Rent::get()?;
        let lamports_amount = rent_sysvar.minimum_balance(data_size);

        let create_instruction = solana_program::system_instruction::create_account(
            initializer_account.key,
            program_pda_account.key,
            lamports_amount,
            data_size as u64,
            program_id,
        );

        invoke_signed(
            &create_instruction,
            &[
                initializer_account.clone(),
                program_pda_account.clone(),
                system_program_account.clone(),
            ],
            &[&program_signature[..]],
        )?;

        if program_token_account.lamports() == 0 {
            msg!(format!("Creating ATA account `{}` for program PDA `{}` to hold Clash tokens because it does not exists yet",
                program_token_account.key, program_pda_account.key
            ).as_str());

            let create_ata_instruction =
                &spl_associated_token_account::create_associated_token_account(
                    initializer_account.key,
                    program_pda_account.key,
                    clash_token_account.key,
                );

            invoke(
                &create_ata_instruction,
                &[
                    initializer_account.clone(),
                    program_pda_account.clone(),
                    program_token_account.clone(),
                    clash_token_account.clone(),
                    system_program_account.clone(),
                    token_program_account.clone(),
                    associated_token_account_program.clone(),
                    sysvar_rent_program_account.clone(),
                ],
            )?;

            msg!(format!(
                "Success creating ATA account `{}` for program PDA account `{}`",
                program_token_account.key, program_pda_account.key
            )
            .as_str());
        }

        // Update ICO data with initializer information
        let mut ico_data = ICOProgramData::try_from_slice(&program_pda_account.data.borrow())?;

        ico_data.initializer = *initializer_account.key;
        ico_data.initializer_ata = *initializer_token_account.key;

        ico_data.serialize(&mut &mut program_pda_account.data.borrow_mut()[..])?;

        msg!(format!(
            "Clash ICO program initialized by `{}`.",
            initializer_account.key
        )
        .as_str());

        Ok(())
    }

    pub fn exchange_clash_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &ClashTokenExchangeData,
    ) -> ProgramResult {
        msg!("Processing exchange SOL by CLASH tokens instruction.");

        // Get accounts
        let accounts_iter = &mut accounts.iter();

        let from_sol_account = next_account_info(accounts_iter)?;
        let to_token_account = next_account_info(accounts_iter)?;

        let to_sol_account = next_account_info(accounts_iter)?;
        let from_token_account = next_account_info(accounts_iter)?;

        let clash_token_account = next_account_info(accounts_iter)?;

        let program_account = next_account_info(accounts_iter)?;
        let program_pda_account = next_account_info(accounts_iter)?;

        let system_program_account = next_account_info(accounts_iter)?;
        let token_program_account = next_account_info(accounts_iter)?;
        let associated_token_account_program = next_account_info(accounts_iter)?;
        let sysvar_rent_program_account = next_account_info(accounts_iter)?;

        validate_account(from_sol_account, true, true, true)?;
        validate_account(to_token_account, false, true, false)?;

        validate_account(to_sol_account, false, true, true)?;
        validate_account(from_token_account, false, true, true)?;

        validate_account(clash_token_account, false, false, true)?;

        if program_account.key != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        if clash_token_account.key != &CLASH_TOKEN_ID {
            ico_err(ICOError::InvalidClashTokenId)?;
        }

        if from_sol_account.key == to_sol_account.key {
            ico_err(ICOError::CannotTransferSameAccount)?;
        }

        if from_token_account.key == to_token_account.key {
            ico_err(ICOError::CannotTransferSameAssociatedAccount)?;
        }

        if to_sol_account.key != &CLASH_SOL_WALLET {
            ico_err(ICOError::InvalidClashTokenDestinationWallet)?;
        }

        if to_token_account.lamports() != 0 {
            let to_associated_token_account =
                TokenAccount::unpack_unchecked(&to_token_account.data.borrow())?;

            if &to_associated_token_account.owner != from_sol_account.key {
                ico_err(ICOError::InvalidSourceAssociatedAccountOwner)?;
            }
        }

        let (program_pda, bump_seed) =
            Pubkey::find_program_address(&[PROGRAM_PDA_SEED1, PROGRAM_PDA_SEED2], program_id);

        let program_signature = &[&PROGRAM_PDA_SEED1, &PROGRAM_PDA_SEED2, &[bump_seed][..]];

        if program_pda_account.key != &program_pda {
            ico_err(ICOError::InvalidAddressProgramPDA)?;
        }

        let from_associated_token_account =
            TokenAccount::unpack_unchecked(&from_token_account.data.borrow())?;

        if from_associated_token_account.owner != program_pda {
            ico_err(ICOError::InvalidProgramAssociatedPDAOwner)?;
        }

        // Calculate outcome value in Clash tokens based on SOL/USD price
        let lamports_amount = data.sol_as_lamports_amount;
        let sol_amount = lamports_amount as f64 / LAMPORTS_PER_SOL as f64;

        let usd_amount = sol_amount * SOL_USD;
        let clash_amount = usd_amount / CLASH_USD;

        if usd_amount < MIN_USD_PRICE {
            ico_err(ICOError::InvalidOfferTooFew)?;
        }

        if usd_amount > MAX_USD_PRICE {
            ico_err(ICOError::InvalidOfferTooMuch)?;
        }

        let clash_mint_data = Mint::unpack_unchecked(&clash_token_account.data.borrow())?;
        let clash_decimals = clash_mint_data.decimals;

        let clash_amount_final: u64 = if clash_decimals > 0 {
            (clash_amount * (10u32.pow(clash_decimals as u32)) as f64) as u64
        } else {
            clash_amount as u64
        };

        // Check exchange can proceed base on CLASH token amount calculated
        if clash_amount_final == 0 {
            ico_err(ICOError::InvalidClashTokenAmount)?;
        }

        // Check for enough funds for both SOL and CLASH token wallets
        if from_sol_account.lamports() <= lamports_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        if from_associated_token_account.amount < clash_amount_final {
            ico_err(ICOError::InsuficientClashToken)?;
        }

        if to_token_account.lamports() == 0 {
            msg!(format!(
                "Creating ATA account `{}` because it does not exists yet",
                to_token_account.key
            )
            .as_str());

            let create_ata_instruction =
                &spl_associated_token_account::create_associated_token_account(
                    from_sol_account.key,
                    from_sol_account.key,
                    clash_token_account.key,
                );

            invoke(
                &create_ata_instruction,
                &[
                    from_sol_account.clone(),
                    to_token_account.clone(),
                    clash_token_account.clone(),
                    system_program_account.clone(),
                    associated_token_account_program.clone(),
                    sysvar_rent_program_account.clone(),
                ],
            )?;

            msg!(format!(
                "Success creating ATA account `{}` for account `{}`",
                to_token_account.key, from_sol_account.key
            )
            .as_str());
        }

        msg!(format!(
            "Exchanging {} SOL tokens({}USD) by {} CLASH tokens from account `{}` to `{}`",
            sol_amount, usd_amount, clash_amount, from_sol_account.key, to_sol_account.key
        )
        .as_str());

        // Transfer SOL as lamports to CLASH team account
        let transfer_instruction =
            transfer(from_sol_account.key, to_sol_account.key, lamports_amount);

        invoke(
            &transfer_instruction,
            &[
                from_sol_account.clone(),
                to_sol_account.clone(),
                system_program_account.clone(),
            ],
        )?;

        msg!(format!(
            "Success transferred {} lamports from `{}` to `{}`.",
            lamports_amount, from_sol_account.key, to_sol_account.key
        )
        .as_str());

        // Transfer CLASH tokens from program ATA to account transferring SOL's
        let transfer_token_instruction = spl_token::instruction::transfer_checked(
            token_program_account.key, // token_program_id: &Pubkey
            from_token_account.key,    // source_pubkey: &Pubkey
            clash_token_account.key,   // mint_pubkey: &Pubkey
            to_token_account.key,      // destination_pubkey: &Pubkey
            program_pda_account.key,   // authority_pubkey: &Pubkey
            &[],                       // signer_pubkeys: &[&Pubkey]
            clash_amount_final,        // amount: u64
            clash_decimals,            // decimals: u8
        )?;

        invoke_signed(
            &transfer_token_instruction,
            &[
                from_sol_account.clone(),
                from_token_account.clone(),
                to_sol_account.clone(),
                to_token_account.clone(),
                clash_token_account.clone(),
                token_program_account.clone(),
                program_pda_account.clone(),
            ],
            &[&program_signature[..]],
        )?;

        msg!(format!(
            "Success transferring {} CLASH tokens from `{}` to `{}`.",
            clash_amount, from_token_account.key, to_token_account.key,
        )
        .as_str());

        Ok(())
    }

    pub fn execute_clash_payment(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &ClashTokenPaymentData,
    ) -> ProgramResult {
        msg!("Processing payment of CLASH tokens payed via Coinpayment");

        // Get accounts
        let accounts_iter = &mut accounts.iter();

        let payer_account = next_account_info(accounts_iter)?;
        let payer_token_account = next_account_info(accounts_iter)?;

        let clash_token_account = next_account_info(accounts_iter)?;

        let trusted_signer_authority = next_account_info(accounts_iter)?;
        let program_token_account = next_account_info(accounts_iter)?;

        let program_account = next_account_info(accounts_iter)?;
        let program_pda_account = next_account_info(accounts_iter)?;

        let system_program_account = next_account_info(accounts_iter)?;
        let token_program_account = next_account_info(accounts_iter)?;
        let associated_token_account_program = next_account_info(accounts_iter)?;
        let sysvar_rent_program_account = next_account_info(accounts_iter)?;

        validate_account(payer_account, false, false, true)?;
        validate_account(payer_token_account, false, true, false)?;

        validate_account(clash_token_account, false, false, true)?;

        validate_account(trusted_signer_authority, true, false, true)?;
        validate_account(program_token_account, false, true, true)?;

        if program_account.key != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        if clash_token_account.key != &CLASH_TOKEN_ID {
            ico_err(ICOError::InvalidClashTokenId)?;
        }

        if payer_token_account.key == program_token_account.key {
            ico_err(ICOError::CannotTransferSameAssociatedAccount)?;
        }

        if trusted_signer_authority.key != &CLASH_PAYMENT_AUTHORITY {
            ico_err(ICOError::InvalidClashTrustedAuthority)?;
        }

        if payer_token_account.lamports() != 0 {
            let payer_associated_token_account =
                TokenAccount::unpack_unchecked(&payer_token_account.data.borrow())?;

            if &payer_associated_token_account.owner != payer_account.key {
                ico_err(ICOError::InvalidClashTokenDestinationWallet)?;
            }
        }

        let (program_pda, bump_seed) =
            Pubkey::find_program_address(&[PROGRAM_PDA_SEED1, PROGRAM_PDA_SEED2], program_id);

        let program_signature = &[&PROGRAM_PDA_SEED1, &PROGRAM_PDA_SEED2, &[bump_seed][..]];

        if program_pda_account.key != &program_pda {
            ico_err(ICOError::InvalidAddressProgramPDA)?;
        }

        let program_associated_token_account =
            TokenAccount::unpack_unchecked(&program_token_account.data.borrow())?;

        if program_associated_token_account.owner != program_pda {
            ico_err(ICOError::InvalidProgramAssociatedPDAOwner)?;
        }

        let clash_amount_final = data.clash_token_amount;

        // Verify for wrong values
        if clash_amount_final == 0 {
            ico_err(ICOError::InvalidClashTokenAmount)?;
        }

        // Check for enough funds for both SOL and CLASH token wallets
        if program_associated_token_account.amount < clash_amount_final {
            ico_err(ICOError::InsuficientClashToken)?;
        }

        if payer_token_account.lamports() == 0 {
            msg!(format!(
                "Creating ATA account `{}` because it does not exists yet",
                payer_token_account.key
            )
            .as_str());

            let create_ata_instruction =
                &spl_associated_token_account::create_associated_token_account(
                    payer_account.key,
                    payer_account.key,
                    clash_token_account.key,
                );

            invoke(
                &create_ata_instruction,
                &[
                    payer_account.clone(),
                    payer_token_account.clone(),
                    clash_token_account.clone(),
                    system_program_account.clone(),
                    associated_token_account_program.clone(),
                    sysvar_rent_program_account.clone(),
                ],
            )?;

            msg!(format!(
                "Success creating ATA account `{}` for account `{}`",
                payer_token_account.key, payer_account.key
            )
            .as_str());
        }

        let clash_amount = clash_amount_final as f64 / LAMPORTS_PER_SOL as f64;

        msg!(format!(
            "Transferring {} CLASH tokens to account `{}` for its payment via CoinPayment",
            clash_amount, payer_token_account.key,
        )
        .as_str());

        let clash_mint_data = Mint::unpack_unchecked(&clash_token_account.data.borrow())?;
        let clash_decimals = clash_mint_data.decimals;

        // Transfer CLASH tokens from program ATA to account transferring SOL's
        let transfer_token_instruction = spl_token::instruction::transfer_checked(
            token_program_account.key, // token_program_id: &Pubkey
            program_token_account.key, // source_pubkey: &Pubkey
            clash_token_account.key,   // mint_pubkey: &Pubkey
            payer_token_account.key,   // destination_pubkey: &Pubkey
            program_pda_account.key,   // authority_pubkey: &Pubkey
            &[],                       // signer_pubkeys: &[&Pubkey]
            clash_amount_final,        // amount: u64
            clash_decimals,            // decimals: u8
        )?;

        invoke_signed(
            &transfer_token_instruction,
            &[
                program_token_account.clone(),
                payer_token_account.clone(),
                clash_token_account.clone(),
                token_program_account.clone(),
                program_pda_account.clone(),
            ],
            &[&program_signature[..]],
        )?;

        msg!(format!(
            "Success transferring {} CLASH tokens from `{}` to `{}`.",
            clash_amount, program_token_account.key, payer_token_account.key,
        )
        .as_str());

        Ok(())
    }

    pub fn terminate_ico(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        msg!("Terminating Clash ICO accounts and metadata.");

        // Get accounts
        let accounts_iter = &mut accounts.iter();

        let initializer_account = next_account_info(accounts_iter)?;
        let initializer_token_account = next_account_info(accounts_iter)?;

        let clash_token_account = next_account_info(accounts_iter)?;

        let program_pda_account = next_account_info(accounts_iter)?;
        let program_token_account = next_account_info(accounts_iter)?;

        let token_program_account = next_account_info(accounts_iter)?;

        validate_account(initializer_account, true, false, true)?;
        validate_account(initializer_token_account, false, false, true)?;

        validate_account(clash_token_account, false, true, true)?;

        validate_account(program_pda_account, false, true, false)?;
        validate_account(program_token_account, false, true, false)?;

        if clash_token_account.key != &CLASH_TOKEN_ID {
            ico_err(ICOError::InvalidClashTokenId)?;
        }

        let (program_pda, bump_seed) =
            Pubkey::find_program_address(&[PROGRAM_PDA_SEED1, PROGRAM_PDA_SEED2], program_id);

        let program_signature = &[&PROGRAM_PDA_SEED1, &PROGRAM_PDA_SEED2, &[bump_seed][..]];

        if program_pda_account.key != &program_pda {
            ico_err(ICOError::InvalidAddressProgramPDA)?;
        };

        if program_pda_account.lamports() == 0 {
            ico_err(ICOError::InvalidTerminateUninitializedICO)?;
        }

        let ico_data = ICOProgramData::try_from_slice(&program_pda_account.data.borrow())?;
        msg!(format!(
            "Terminating an ICO initialized by `{}`",
            ico_data.initializer
        )
        .as_str());

        if &ico_data.initializer != initializer_account.key {
            ico_err(ICOError::InitializerAccountMismatch)?;
        }

        if &ico_data.initializer_ata != initializer_token_account.key {
            ico_err(ICOError::InitializerAssociatedAccountMismatch)?;
        }

        let clash_mint_data = Mint::unpack_unchecked(&clash_token_account.data.borrow())?;

        if program_token_account.lamports() != 0 {
            msg!(format!(
                "Closing ATA account `{}` from program PDA `{}` and returning {} lamports to initializer account",
                program_token_account.key, program_pda_account.key, program_token_account.lamports()
            )
            .as_str());

            let program_associated_token_account =
                TokenAccount::unpack_unchecked(&program_token_account.data.borrow())?;

            validate_token_account(
                &program_associated_token_account,
                program_pda_account.key,
                &CLASH_TOKEN_ID,
            )?;

            let amount_clash: u64 = program_associated_token_account.amount;
            let decimals: u8 = clash_mint_data.decimals;

            if amount_clash > 0 {
                msg!(format!(
                    "Transferring {} remaining Clash tokens to initializer Clash associated token account",
                    amount_clash
                )
                .as_str());

                let transfer_token_instruction = spl_token::instruction::transfer_checked(
                    token_program_account.key,     // token_program_id: &Pubkey
                    program_token_account.key,     // source_pubkey: &Pubkey
                    clash_token_account.key,       // mint_pubkey: &Pubkey
                    initializer_token_account.key, // destination_pubkey: &Pubkey
                    program_pda_account.key,       // authority_pubkey: &Pubkey
                    &[],                           // signer_pubkeys: &[&Pubkey]
                    amount_clash,                  // amount: u64
                    decimals,                      // decimals: u8
                )?;

                invoke_signed(
                    &transfer_token_instruction,
                    &[
                        program_token_account.clone(),
                        clash_token_account.clone(),
                        initializer_token_account.clone(),
                        token_program_account.clone(),
                        program_pda_account.clone(),
                    ],
                    &[&program_signature[..]],
                )?;
            }

            let close_instruction = spl_token::instruction::close_account(
                token_program_account.key, // token_program_id: &Pubkey
                program_token_account.key, // account_pubkey: &Pubkey
                initializer_account.key,   // destination_pubkey: &Pubkey
                program_pda_account.key,   // owner_pubkey: &Pubkey
                &[],                       // signer_pubkeys: &[&Pubkey]
            )?;

            invoke_signed(
                &close_instruction,
                &[
                    program_token_account.clone(),
                    initializer_account.clone(),
                    program_pda_account.clone(),
                    token_program_account.clone(),
                ],
                &[&program_signature[..]],
            )?;

            msg!("Success closing Clash associated token account owned by the ICO program.");
        }

        msg!(format!(
            "Closing program account `{}`(PDA) and sending lamports back to initializer",
            program_pda_account.key
        )
        .as_str());

        let lamports_amount = program_pda_account.lamports();

        msg!(format!(
            "Transferring {} lamports back to initializer account",
            &lamports_amount
        )
        .as_str());

        **program_pda_account.try_borrow_mut_lamports()? -= lamports_amount;
        **initializer_account.try_borrow_mut_lamports()? += lamports_amount;

        msg!("ICO has been terminated.");

        Ok(())
    }
}
