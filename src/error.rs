use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StakeError {
    #[error("Account not initialized yet")]
    UninitializedAccount,
    #[error("PDA passed & PDA derived do not match")]
    InvalidPda,
    #[error("Invalid token account passed")]
    InvalidTokenAccount,
    #[error("Invalid stake account passed")]
    InvalidStakeAccount,
}

impl From<StakeError> for ProgramError {
    fn from(e: StakeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
