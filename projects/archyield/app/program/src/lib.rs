use arch_program::{
    account::AccountInfo,
    clock::Clock,
    entrypoint,
    instruction::Instruction,
    msg,
    program::{invoke, next_account_info},
    program_error::ProgramError,
    pubkey::Pubkey,
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
    pub last_reward_claim_time: u64,
    pub reward: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum DepositStatus {
    Active,
    Withdrawn,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolList {
    pub pools: Vec<Pubkey>,
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
    let pool_list_account = next_account_info(account_iter)?;

    let mut pool_list: PoolList = match PoolList::try_from_slice(&pool_list_account.data.borrow()) {
        Ok(list) => list,
        Err(_) => PoolList { pools: Vec::new() },
    };

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

    pool_list.pools.push(*pool_account.key);

    pool_list
        .serialize(&mut &mut pool_list_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
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
    let token_program = next_account_info(account_iter)?;
    let user_token_account = next_account_info(account_iter)?;
    let pool_token_account = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;

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

    let transfer_accounts = &[
        user_account.clone(),
        token_mint.clone(),
        user_token_account.clone(),
        pool_token_account.clone(),
    ];

    let mut instruction_data = vec![];
    instruction_data.extend_from_slice(token_program.key.as_ref());
    instruction_data.extend_from_slice(token_mint.key.as_ref());
    instruction_data.extend_from_slice(user_token_account.key.as_ref());
    instruction_data.extend_from_slice(pool_token_account.key.as_ref());
    instruction_data.push(3);
    instruction_data.extend_from_slice(&deposit_amount.to_le_bytes());

    let transfer_instruction = Instruction::from_slice(&instruction_data);

    invoke(&transfer_instruction, transfer_accounts)?;

    let mut pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let user_deposit: Option<Deposits> = match Deposits::try_from_slice(&user_account.data.borrow())
    {
        Ok(deposit) => Some(deposit),
        Err(_) => None,
    };
    let apy = pool.apy;
    let days_in_year = 365;
    let daily_payout = (deposit_amount * apy / 100) / days_in_year;

    let updated_deposit = if let Some(mut existing) = user_deposit {
        existing.deposited_amount = existing
            .deposited_amount
            .checked_add(deposit_amount)
            .ok_or(ProgramError::InvalidAccountData)?;
        existing.reward = existing.reward;
        existing.daily_payout = (existing.deposited_amount * apy / 100) / days_in_year;
        existing.start_date = Clock::default().unix_timestamp as u64;
        let elapsed_days: u64;
        if existing.last_reward_claim_time == 0 {
            let current_time = Clock::default().unix_timestamp as u64;
            elapsed_days = (current_time - existing.start_date) / (24 * 60 * 60);
        } else {
            let current_time = Clock::default().unix_timestamp as u64;
            elapsed_days = (current_time - existing.last_reward_claim_time) / (24 * 60 * 60);
        }
        let base_reward = existing.daily_payout * elapsed_days;
        let compound_reward = if existing.reward > 0 {
            ((existing.reward * apy / 100) / days_in_year) * elapsed_days
        } else {
            0
        };
        existing.reward = existing.reward + base_reward + compound_reward;
        existing
    } else {
        Deposits {
            user_pubkey: *user_account.key,
            pool_pubkey: *pool_account.key,
            deposited_amount: deposit_amount,
            status: DepositStatus::Active,
            daily_payout,
            start_date: Clock::default().unix_timestamp as u64,
            last_reward_claim_time: 0,
            reward: 0,
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
    let token_program = next_account_info(account_iter)?;
    let pool_token_account = next_account_info(account_iter)?;
    let user_token_account = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;

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

    let clock = Clock::default();
    let current_time = clock.unix_timestamp as u64;
    if current_time < user_deposit.start_date + pool.min_period {
        return Err(ProgramError::Custom(1));
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let withdraw_amount = user_deposit.deposited_amount;
    let transfer_accounts = &[
        pool_account.clone(),
        token_mint.clone(),
        pool_token_account.clone(),
        user_token_account.clone(),
    ];

    let mut instruction_data = vec![];
    instruction_data.extend_from_slice(token_program.key.as_ref());
    instruction_data.extend_from_slice(pool_token_account.key.as_ref());
    instruction_data.extend_from_slice(pool_account.key.as_ref());
    instruction_data.extend_from_slice(user_token_account.key.as_ref());
    instruction_data.push(3);
    instruction_data.extend_from_slice(&withdraw_amount.to_le_bytes());

    let transfer_instruction = Instruction::from_slice(&instruction_data);

    invoke(&transfer_instruction, transfer_accounts)?;

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

pub fn withdraw_rewards(program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let user_account = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let pool_token_account = next_account_info(account_iter)?;
    let user_token_account = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;

    let pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    let mut deposit = Deposits::try_from_slice(&user_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if pool_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if deposit.reward == 0 {
        return Err(ProgramError::Custom(0)); // No rewards to withdraw
    }

    let days_in_year = 365;
    let elapsed_days: u64;
    if deposit.last_reward_claim_time == 0 {
        let current_time = Clock::default().unix_timestamp as u64;
        elapsed_days = (current_time - deposit.start_date) / (24 * 60 * 60);
    } else {
        let current_time = Clock::default().unix_timestamp as u64;
        elapsed_days = (current_time - deposit.last_reward_claim_time) / (24 * 60 * 60);
    }
    deposit.reward = deposit.daily_payout * elapsed_days;
    let reward_amount =
        (((deposit.reward * pool.apy / 100) / days_in_year) * elapsed_days) + deposit.reward;
    deposit.reward = 0;
    deposit.last_reward_claim_time = Clock::default().unix_timestamp as u64;

    let transfer_accounts = &[
        pool_account.clone(),
        token_mint.clone(),
        pool_token_account.clone(),
        user_token_account.clone(),
    ];

    let mut instruction_data = vec![];
    instruction_data.extend_from_slice(token_program.key.as_ref());
    instruction_data.extend_from_slice(pool_token_account.key.as_ref());
    instruction_data.extend_from_slice(pool_account.key.as_ref());
    instruction_data.extend_from_slice(user_token_account.key.as_ref());
    instruction_data.push(3);
    instruction_data.extend_from_slice(&reward_amount.to_le_bytes());

    let transfer_instruction = Instruction::from_slice(&instruction_data);

    invoke(&transfer_instruction, transfer_accounts)?;

    deposit.reward = 0;
    deposit.last_reward_claim_time = Clock::default().unix_timestamp as u64;

    deposit
        .serialize(&mut &mut user_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!("Reward withdrawal successful: {}", reward_amount);
    Ok(())
}

pub fn get_user_deposit(accounts: &[AccountInfo]) -> Result<Deposits, ProgramError> {
    let account_iter = &mut accounts.iter();
    let pool_account = next_account_info(account_iter)?;
    let user_account = next_account_info(account_iter)?;

    let pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    let mut user_deposit: Deposits = Deposits::try_from_slice(&user_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    let days_in_year = 365;
    let elapsed_days: u64;
    if user_deposit.last_reward_claim_time == 0 {
        let current_time = Clock::default().unix_timestamp as u64;
        elapsed_days = (current_time - user_deposit.start_date) / (24 * 60 * 60);
    } else {
        let current_time = Clock::default().unix_timestamp as u64;
        elapsed_days = (current_time - user_deposit.last_reward_claim_time) / (24 * 60 * 60);
    }
    user_deposit.reward = user_deposit.daily_payout * elapsed_days;
    let reward_payout = ((user_deposit.reward * pool.apy / 100) / days_in_year) * elapsed_days;
    user_deposit.reward = reward_payout;

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

pub fn get_all_pools(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<Vec<Pool>, ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_list_account = next_account_info(account_iter)?;

    if pool_list_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let pool_list: PoolList = PoolList::try_from_slice(&pool_list_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let mut pools: Vec<Pool> = Vec::new();

    for pool_pubkey in pool_list.pools {
        let pool_account = accounts
            .iter()
            .find(|acc| acc.key == &pool_pubkey)
            .ok_or(ProgramError::InvalidAccountData)?;

        let pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        pools.push(pool);
    }

    Ok(pools)
}
