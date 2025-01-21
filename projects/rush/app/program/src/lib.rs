use arch_program::{
    account::AccountInfo, clock::Clock, entrypoint, msg, program::next_account_info,
    program_error::ProgramError, pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct User {
    pub user_pubkey: Pubkey,
    pub spins: u64,
    pub last_spin_time: u64,
}

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    match instruction_data[0] {
        0 => create_user(program_id, accounts, &instruction_data[1..]),
        1 => start(program_id, accounts, &instruction_data[1..]),
        2 => end_game(program_id, accounts, &instruction_data[1..]),
        3 => get_user(accounts, &instruction_data[1..]),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

pub fn create_user(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let user_account = next_account_info(account_iter)?;

    let user = match User::try_from_slice(&user_account.data.borrow()) {
        Ok(_) => return Err(ProgramError::AccountAlreadyInitialized),
        Err(_) => User {
            user_pubkey: *user_account.key,
            spins: 10,
            last_spin_time: 0,
        },
    };

    // pool.serialize(&mut &mut pool_account.data.borrow_mut()[..])
    //     .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!("User profile created successfully: {:?}", user);
    Ok(())
}

pub fn start(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let user_account = next_account_info(account_iter)?;

    let mut user = match User::try_from_slice(&user_account.data.borrow()) {
        Ok(res) => res,
        Err(_) => return Err(ProgramError::InvalidAccountData),
    };

    if user.last_spin_time == 0 {
        user.spins = 10;
    }

    let clock = Clock::default();
    let ten_hrs = clock.unix_timestamp as u64;
    let one_hr = clock.unix_timestamp as u64;

    if user.last_spin_time > 0 {
        let diff = clock.unix_timestamp as u64 - user.last_spin_time;
        if diff > ten_hrs {
            user.spins = 10;
        } else {
            user.spins = diff / one_hr;
        }
    }

    msg!("Starting game for user: {:?}", user);
    Ok(())
}

pub fn end_game(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let user_account = next_account_info(account_iter)?;

    let mut user = match User::try_from_slice(&user_account.data.borrow()) {
        Ok(res) => res,
        Err(_) => return Err(ProgramError::InvalidAccountData),
    };

    let clock = Clock::default();
    user.last_spin_time = clock.unix_timestamp as u64;

    user.serialize(&mut &mut user_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!("Game ended: {:?}", user);
    Ok(())
}

pub fn get_user(accounts: &[AccountInfo], _instruction_data: &[u8]) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();
    let user_account = next_account_info(account_iter)?;

    let user = match User::try_from_slice(&user_account.data.borrow()) {
        Ok(res) => res,
        Err(_) => return Err(ProgramError::InvalidAccountData),
    };

    msg!("User: {:?}", user);

    Ok(())
}
