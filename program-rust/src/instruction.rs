use solana_program::program_error::ProgramError;

use crate::error::{ico_err, ICOError};

use crate::state::{ClashTokenExchangeData, ClashTokenPaymentData};

use borsh::BorshDeserialize;

#[derive(PartialEq)]
pub enum ProgramInstruction {
    InitializeICO,
    ExchangeClashToken { data: ClashTokenExchangeData },
    ExecuteClashPayment { data: ClashTokenPaymentData },
    TerminateICO,

    // Internal usage only
    InvalidInstruction,
}

impl ProgramInstruction {
    pub fn unpack(input_data: &[u8]) -> Result<Self, ProgramError> {
        if input_data.is_empty() {
            ico_err(ICOError::InvalidInstructionDataEmpty)?;
        }

        let instruction_type: u8 = input_data[0];
        let instruction_data: &[u8] = &input_data[1..];

        let instruction: ProgramInstruction = match instruction_type {
            0 => ProgramInstruction::InitializeICO,
            1 => ProgramInstruction::ExchangeClashToken {
                data: ClashTokenExchangeData::try_from_slice(instruction_data)?,
            },
            2 => ProgramInstruction::ExecuteClashPayment {
                data: ClashTokenPaymentData::try_from_slice(instruction_data)?,
            },
            3 => ProgramInstruction::TerminateICO,
            _ => ProgramInstruction::InvalidInstruction,
        };

        if instruction == ProgramInstruction::InvalidInstruction {
            ico_err(ICOError::InvalidProgramInstruction)?;
        }

        Ok(instruction)
    }
}
