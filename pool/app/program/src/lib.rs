use arch_program::{
    account::AccountInfo, clock::Clock, entrypoint, msg, program::next_account_info,
    program_error::ProgramError, pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Pool {
    pub pool_pubkey: Pubkey,   // Unique identifier for the pool
    pub pool_name: String,     // Optional, for human-readable naming
    pub risk_type: RiskType,   // Custom enum for risk classification
    pub apy: u64,              // Annual Percentage Yield
    pub min_period: u64,       // Minimum coverage period
    pub total_unit: u64,       // Total cover units
    pub tvl: u64,              // Total value locked
    pub base_value: u64,       // Base valuation of the pool
    pub cover_units: u64,      // Units of cover provided
    pub tcp: u64,              // Total claimable pool
    pub is_active: bool,       // Status of the pool
    pub asset_pubkey: Pubkey,  // Pubkey for the associated asset
    pub asset_type: AssetType, // Enum for asset type (BTC, etc.)
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub enum AssetType {
    BTC,
    Runes,
    Ordinals,
}

impl AssetType {
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(AssetType::BTC),
            1 => Ok(AssetType::Runes),
            2 => Ok(AssetType::Ordinals),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub enum RiskType {
    Low,
    Medium,
    High,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Deposits {
    pub user_pubkey: Pubkey,
    pub pool_pubkey: Pubkey,
    pub deposited_amount: u64,
    pub status: DepositStatus,
    pub daily_payout: u64,
    pub start_date: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum DepositStatus {
    Active,
    Withdrawn,
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
        0 => create_pool(program_id, accounts, instruction_data),
        1 => deposit(program_id, accounts, instruction_data),
        2 => withdraw(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

pub fn create_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    if pool_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if instruction_data.len() < 16 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let pool_name_len = instruction_data[0] as usize;
    if instruction_data.len() < 25 + pool_name_len {
        return Err(ProgramError::InvalidInstructionData);
    }

    let pool_name = String::from_utf8(instruction_data[1..1 + pool_name_len].to_vec())
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let asset_type = AssetType::from_u8(instruction_data[1 + pool_name_len])?;
    let apy = u64::from_le_bytes(
        instruction_data[2 + pool_name_len..10 + pool_name_len]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    let min_period = u64::from_le_bytes(
        instruction_data[10 + pool_name_len..18 + pool_name_len]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    let base_value = u64::from_le_bytes(
        instruction_data[18 + pool_name_len..26 + pool_name_len]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    let pool = Pool {
        pool_pubkey: *pool_account.key,
        pool_name,
        risk_type: RiskType::Low,
        apy,
        min_period,
        total_unit: 0,
        tvl: 0,
        base_value,
        cover_units: 0,
        tcp: 0,
        is_active: true,
        asset_pubkey: *pool_account.key,
        asset_type,
    };

    pool.serialize(&mut &mut pool_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!("Pool created successfully: {:?}", pool);
    Ok(())
}

pub fn deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let user_account = next_account_info(account_iter)?;

    if pool_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let deposit_amount = u64::from_le_bytes(
        instruction_data[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    let mut pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let user_deposit: Option<Deposits> = None;
    let apy = pool.apy;
    let days_in_year = 365;
    let daily_payout = (deposit_amount * apy / 100) / days_in_year;

    let updated_deposit = if let Some(mut existing) = user_deposit {
        existing.deposited_amount = existing
            .deposited_amount
            .checked_add(deposit_amount)
            .ok_or(ProgramError::InvalidAccountData)?;
        existing.daily_payout = (existing.deposited_amount * apy / 100) / days_in_year;
        existing.start_date = Clock::default().unix_timestamp as u64;
        existing
    } else {
        Deposits {
            user_pubkey: *user_account.key,
            pool_pubkey: *pool_account.key,
            deposited_amount: deposit_amount,
            status: DepositStatus::Active,
            daily_payout,
            start_date: Clock::default().unix_timestamp as u64,
        }
    };

    updated_deposit
        .serialize(&mut &mut user_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    pool.tvl = pool
        .tvl
        .checked_add(deposit_amount)
        .ok_or(ProgramError::InvalidAccountData)?;

    pool.total_unit = pool
        .total_unit
        .checked_add(deposit_amount)
        .ok_or(ProgramError::InvalidAccountData)?;

    pool.serialize(&mut &mut pool_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!(
        "Deposit successful. Amount: {}, New TVL: {}",
        deposit_amount,
        pool.tvl
    );
    Ok(())
}

pub fn withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let user_account = next_account_info(account_iter)?;

    if pool_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    let mut user_deposit: Deposits = Deposits::try_from_slice(&user_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    if user_deposit.deposited_amount == 0 {
        return Err(ProgramError::InvalidAccountData); // No deposit found
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let withdraw_amount = user_deposit.deposited_amount;
    user_deposit.deposited_amount = 0;
    user_deposit.status = DepositStatus::Withdrawn;

    if withdraw_amount > pool.tvl {
        return Err(ProgramError::InsufficientFunds);
    }

    pool.tvl = pool
        .tvl
        .checked_sub(withdraw_amount)
        .ok_or(ProgramError::InvalidAccountData)?;

    pool.total_unit = pool
        .total_unit
        .checked_sub(withdraw_amount)
        .ok_or(ProgramError::InvalidAccountData)?;

    pool.serialize(&mut &mut pool_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!(
        "Withdraw successful. Amount: {}, Remaining TVL: {}",
        withdraw_amount,
        pool.tvl
    );
    Ok(())
}

pub fn get_user_deposit(accounts: &[AccountInfo]) -> Result<Deposits, ProgramError> {
    let account_iter = &mut accounts.iter();
    let user_account = next_account_info(account_iter)?;

    let user_deposit: Deposits = Deposits::try_from_slice(&user_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    Ok(user_deposit)
}

pub fn get_pool_tvl(accounts: &[AccountInfo]) -> Result<u64, ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    Ok(pool.tvl)
}

pub fn get_pool(accounts: &[AccountInfo]) -> Result<Pool, ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    Ok(pool)
}
