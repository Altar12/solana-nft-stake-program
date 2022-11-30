use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    clock::UnixTimestamp,
    program_pack::{IsInitialized, Sealed},
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct UserStakeInfo {
    pub is_initialized: bool,
    pub user: Pubkey,
    pub token_account: Pubkey,
    pub stake_start_time: UnixTimestamp,
    pub last_redeem_time: UnixTimestamp,
    pub is_stake_active: bool,
}
impl UserStakeInfo {
    pub const SIZE: usize = 1 + 32 + 32 + 64 + 64 + 1;
}
impl Sealed for UserStakeInfo {}
impl IsInitialized for UserStakeInfo {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
