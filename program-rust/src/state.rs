use solana_program::pubkey::Pubkey;

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct ICOProgramData {
    pub initializer: Pubkey,
    pub initializer_ata: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct ClashTokenExchangeData {
    pub sol_as_lamports_amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub struct ClashTokenPaymentData {
    pub clash_token_amount: u64,
}
