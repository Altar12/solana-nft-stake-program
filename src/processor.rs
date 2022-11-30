use crate::error::StakeError;
use crate::instruction::StakeInstruction;
use crate::state::UserStakeInfo;
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use std::convert::TryInto;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = StakeInstruction::unpack(instruction_data)?;
    match instruction {
        StakeInstruction::InitializeStakeAccount => {
            process_initialize_stake_account(program_id, accounts)
        }
        StakeInstruction::Stake => process_stake(program_id, accounts),
        StakeInstruction::Redeem => process_redeem(program_id, accounts),
        StakeInstruction::Unstake => process_unstake(program_id, accounts),
    }
}

pub fn process_initialize_stake_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let nft_token_account = next_account_info(account_info_iter)?;
    let stake_state = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (stake_state_pda, bump) = Pubkey::find_program_address(
        &[user.key.as_ref(), nft_token_account.key.as_ref()],
        program_id,
    );
    if stake_state_pda != *stake_state.key {
        msg!("Invalid PDA passed");
        return Err(StakeError::InvalidPda.into());
    }
    let space = UserStakeInfo::SIZE;
    let rent_lamports = Rent::get()?.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            user.key,
            stake_state.key,
            rent_lamports,
            space.try_into().unwrap(),
            program_id,
        ),
        &[user.clone(), stake_state.clone(), system_program.clone()],
        &[&[user.key.as_ref(), nft_token_account.key.as_ref(), &[bump]]],
    )?;

    let mut account_data =
        try_from_slice_unchecked::<UserStakeInfo>(&stake_state.data.borrow()).unwrap();
    if account_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    account_data.user = *user.key;
    account_data.token_account = *nft_token_account.key;
    account_data.is_stake_active = false;
    account_data.is_initialized = true;
    account_data.serialize(&mut &mut stake_state.data.borrow_mut()[..])?;
    Ok(())
}

pub fn process_stake(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let nft_token_account = next_account_info(account_info_iter)?;
    let stake_state = next_account_info(account_info_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if stake_state.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    let (pda, _bump) = Pubkey::find_program_address(
        &[user.key.as_ref(), nft_token_account.key.as_ref()],
        program_id,
    );
    if pda != *stake_state.key {
        msg!("Invalid PDA seeds");
        return Err(StakeError::InvalidPda.into());
    }
    let mut account_data =
        try_from_slice_unchecked::<UserStakeInfo>(&stake_state.data.borrow()).unwrap();
    if !account_data.is_initialized() {
        return Err(StakeError::UninitializedAccount.into());
    }
    if account_data.is_stake_active {
        return Err(ProgramError::InvalidArgument);
    }
    let clock = Clock::get()?;
    account_data.user = *user.key;
    account_data.token_account = *nft_token_account.key;
    account_data.stake_start_time = clock.unix_timestamp;
    account_data.last_redeem_time = clock.unix_timestamp;
    account_data.is_stake_active = true;
    account_data.serialize(&mut &mut stake_state.data.borrow_mut()[..])?;
    Ok(())
}

pub fn process_redeem(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let nft_token_account = next_account_info(account_info_iter)?;
    let stake_state = next_account_info(account_info_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if stake_state.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    let (pda, _bump) = Pubkey::find_program_address(
        &[user.key.as_ref(), nft_token_account.key.as_ref()],
        program_id,
    );
    if pda != *stake_state.key {
        return Err(StakeError::InvalidPda.into());
    }
    let mut account_data =
        try_from_slice_unchecked::<UserStakeInfo>(&stake_state.data.borrow()).unwrap();
    if !account_data.is_initialized() {
        return Err(StakeError::UninitializedAccount.into());
    }
    if !account_data.is_stake_active {
        msg!("Not staked yet");
        return Err(ProgramError::InvalidArgument);
    }
    if account_data.user != *user.key {
        return Err(StakeError::InvalidStakeAccount.into());
    }
    if account_data.token_account != *nft_token_account.key {
        return Err(StakeError::InvalidTokenAccount.into());
    }
    let clock = Clock::get()?;
    let reward_amt = clock.unix_timestamp - account_data.last_redeem_time;
    msg!("Reward: {}", reward_amt);
    account_data.last_redeem_time = clock.unix_timestamp;
    account_data.serialize(&mut &mut stake_state.data.borrow_mut()[..])?;
    Ok(())
}

pub fn process_unstake(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user = next_account_info(account_info_iter)?;
    let nft_token_account = next_account_info(account_info_iter)?;
    let stake_state = next_account_info(account_info_iter)?;

    if !user.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if stake_state.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    let (pda, _bump) = Pubkey::find_program_address(
        &[user.key.as_ref(), nft_token_account.key.as_ref()],
        program_id,
    );
    if pda != *stake_state.key {
        return Err(StakeError::InvalidPda.into());
    }
    let mut account_data =
        try_from_slice_unchecked::<UserStakeInfo>(&stake_state.data.borrow()).unwrap();
    if !account_data.is_initialized() {
        return Err(StakeError::UninitializedAccount.into());
    }
    if !account_data.is_stake_active {
        return Err(ProgramError::InvalidArgument);
    }
    if account_data.user != *user.key {
        return Err(StakeError::InvalidStakeAccount.into());
    }
    if account_data.token_account != *nft_token_account.key {
        return Err(StakeError::InvalidTokenAccount.into());
    }
    let clock = Clock::get()?;
    let reward_amt = clock.unix_timestamp - account_data.last_redeem_time;
    msg!("Reward: {}", reward_amt);
    account_data.last_redeem_time = clock.unix_timestamp;
    account_data.is_stake_active = false;
    account_data.serialize(&mut &mut stake_state.data.borrow_mut()[..])?;
    Ok(())
}
