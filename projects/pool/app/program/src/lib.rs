use arch_program::{
    account::{AccountInfo, AccountMeta},
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
    pub pool_id: u64, // Unique identifier for the pool
    pub pool_pubkey: Pubkey,
    pub pool_name: String,   // Optional, for human-readable naming
    pub risk_type: RiskType, // Custom enum for risk classification
    pub apy: u64,            // Annual Percentage Yield
    pub min_period: u64,     // Minimum coverage period
    pub total_unit: u64,     // Total cover units
    pub tvl: u64,            // Total value locked
    pub base_value: u64,     // Base valuation of the pool
    pub investment_arm: u64,
    pub cover_units: u64,      // Units of cover provided
    pub tcp: u64,              // Total claimable pool
    pub is_active: bool,       // Status of the pool
    pub asset_pubkey: Pubkey,  // Pubkey for the associated asset
    pub asset_type: AssetType, // Enum for asset type (BTC, etc.)
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolParam {
    pub pool_name: String,
    pub risk_type: u8,
    pub apy: u64, // Annual Percentage Yield
    pub min_period: u64,
    pub asset_pubkey: Pubkey, // Pubkey for the associated asset
    pub asset_type: u8,
    pub investment_arm: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransferInput {
    pub amount: u64,
}
impl TransferInput {
    pub fn new(amount: u64) -> Self {
        TransferInput { amount }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct DepositParam {
    pool_id: u64,
    amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct QueryParam {
    pool_id: u64,
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

impl RiskType {
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(RiskType::Low),
            1 => Ok(RiskType::Medium),
            2 => Ok(RiskType::High),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Deposits {
    pub pool_id: u64,
    pub user_pubkey: Pubkey,
    pub pool_pubkey: Pubkey,
    pub deposited_amount: u64,
    pub status: DepositStatus,
    pub daily_payout: u64,
    pub start_date: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum DepositStatus {
    Active,
    Withdrawn,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct PoolList {
    pub pools: Vec<u64>,
    pub pool_id_to_pubkey: Vec<(u64, Pubkey)>,
    pub admin_list: Vec<Pubkey>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct UserDepositList {
    pub deposits: Vec<Deposits>,
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
        0 => create_pool(program_id, accounts, &instruction_data[1..]),
        1 => deposit(program_id, accounts, &instruction_data[1..]),
        2 => withdraw(program_id, accounts, &instruction_data[1..]),
        // 3 => get_user_deposit(accounts, &instruction_data[1..]),
        // 4 => get_all_pools(program_id, accounts),
        // 5 => get_pool_by_id(accounts, &instruction_data[1..]),
        // 6 => get_pool_tvl(accounts, &instruction_data[1..])
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

    let mut pool_list = match PoolList::try_from_slice(&pool_list_account.data.borrow()) {
        Ok(list) => list,
        Err(_) => PoolList {
            pools: Vec::new(),
            pool_id_to_pubkey: Vec::new(),
            admin_list: vec![],
        },
    };

    let pool_param = match PoolParam::try_from_slice(instruction_data) {
        Ok(list) => list,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };

    // if pool_account.owner != program_id {
    //     return Err(ProgramError::IncorrectProgramId);
    // }

    if !pool_list.admin_list.contains(owner_account.key) {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let asset_type = AssetType::from_u8(pool_param.asset_type)?;
    let risk_type = RiskType::from_u8(pool_param.risk_type)?;
    let pool_id = pool_list.pools.len() as u64 + 1;

    let pool = Pool {
        pool_id,
        pool_pubkey: *pool_account.key,
        pool_name: pool_param.pool_name,
        risk_type,
        apy: pool_param.apy,
        min_period: pool_param.min_period,
        total_unit: 0,
        investment_arm: pool_param.investment_arm,
        tvl: 0,
        base_value: 0,
        cover_units: 0,
        tcp: 0,
        is_active: true,
        asset_pubkey: pool_param.asset_pubkey,
        asset_type,
    };

    pool_list.pools.push(pool_id);
    pool_list
        .pool_id_to_pubkey
        .push((pool_id, *pool_account.key));

    pool_list
        .serialize(&mut &mut pool_list_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
    pool.serialize(&mut &mut pool_account.data.borrow_mut()[..])
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    msg!(
        "Pool {:?} with ID {:?} created successfully: {:?}",
        pool_account.key,
        pool_id,
        pool
    );
    Ok(())
}

pub fn deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_account = next_account_info(account_iter)?;
    let pool_list_account = next_account_info(account_iter)?;
    let user_account = next_account_info(account_iter)?;
    let token_program = next_account_info(account_iter)?;
    let user_token_account = next_account_info(account_iter)?;
    let pool_token_account = next_account_info(account_iter)?;
    let token_mint = next_account_info(account_iter)?;

    if pool_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let deposit_param = match DepositParam::try_from_slice(instruction_data) {
        Ok(list) => list,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };

    let mut user_deposit_list = match UserDepositList::try_from_slice(&user_account.data.borrow()) {
        Ok(list) => list,
        Err(_) => UserDepositList {
            deposits: Vec::new(),
        },
    };

    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let pool_list = PoolList::try_from_slice(&pool_list_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let deposit_amount = deposit_param.amount;

    let transfer_accounts = &[
        user_account.clone(),
        token_mint.clone(),
        user_token_account.clone(),
        pool_token_account.clone(),
    ];

    let pool_pubkey = pool_list
        .pool_id_to_pubkey
        .iter()
        .find(|(id, _)| *id == deposit_param.pool_id)
        .map(|(_, pubkey)| pubkey);

    let pool = if let Some(pool_pubkey) = pool_pubkey {
        let pool_account = account_iter
            .find(|account| account.key == pool_pubkey)
            .ok_or(ProgramError::InvalidAccountData)?;

        let pool = Pool::try_from_slice(&pool_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

        pool
    } else {
        return Err(ProgramError::InvalidArgument);
    };

    let mut transfer_ix_data = vec![3];
    transfer_ix_data.extend_from_slice(
        &borsh::to_vec(&TransferInput {
            amount: deposit_amount,
        })
        .unwrap(),
    );

    let transfer_ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta {
                pubkey: *user_account.key,
                is_signer: true,   // This account needs to sign the transaction
                is_writable: true, // This account's data will be modified
            },
            AccountMeta {
                pubkey: pool.asset_pubkey,
                is_signer: false,   // Mint doesn't need to sign
                is_writable: false, // Mint data won't be modified
            },
            AccountMeta {
                pubkey: *user_token_account.key,
                is_signer: false,  // Token account doesn't sign
                is_writable: true, // Token account balance will change
            },
            AccountMeta {
                pubkey: *pool_token_account.key,
                is_signer: false,  // Pool token account doesn't sign
                is_writable: true, // Pool token account balance will change
            },
        ],
        data: transfer_ix_data,
    };

    invoke(&transfer_ix, transfer_accounts)?;

    let mut pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let apy = pool.apy;
    let days_in_year = 365;
    let daily_payout = (deposit_amount * apy / 100) / days_in_year;
    if let Some(deposit) = user_deposit_list
        .deposits
        .iter_mut()
        .find(|d| d.pool_id == deposit_param.pool_id)
    {
        deposit.deposited_amount = deposit
            .deposited_amount
            .checked_add(deposit_amount)
            .ok_or(ProgramError::InvalidAccountData)?;
        deposit.daily_payout = (deposit.deposited_amount * apy / 100) / days_in_year;
        deposit.start_date = Clock::default().unix_timestamp as u64;
    } else {
        user_deposit_list.deposits.push(Deposits {
            pool_id: deposit_param.pool_id,
            user_pubkey: *user_account.key,
            pool_pubkey: *pool_account.key,
            deposited_amount: deposit_amount,
            status: DepositStatus::Active,
            daily_payout,
            start_date: Clock::default().unix_timestamp as u64,
        });
    }

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

    let withdraw_param = match QueryParam::try_from_slice(instruction_data) {
        Ok(list) => list,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };
    let mut pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let mut user_deposit_list = match UserDepositList::try_from_slice(&user_account.data.borrow()) {
        Ok(list) => list,
        Err(_) => UserDepositList {
            deposits: Vec::new(),
        },
    };

    let user_deposit = if let Some(deposit) = user_deposit_list
        .deposits
        .iter_mut()
        .find(|d| d.pool_id == withdraw_param.pool_id)
    {
        deposit
    } else {
        return Err(ProgramError::InvalidAccountData);
    };

    let clock = Clock::default();
    let current_time = clock.unix_timestamp as u64;
    if current_time < user_deposit.start_date + pool.min_period {
        return Err(ProgramError::Custom(1));
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let withdraw_amount = user_deposit.deposited_amount;

    if withdraw_amount > pool.tvl {
        return Err(ProgramError::InsufficientFunds);
    }

    let mut transfer_ix_data = vec![3];
    transfer_ix_data.extend_from_slice(
        &borsh::to_vec(&TransferInput {
            amount: withdraw_amount,
        })
        .unwrap(),
    );

    let transfer_ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta {
                pubkey: *pool_account.key,
                is_signer: true,
                is_writable: true,
            },
            AccountMeta {
                pubkey: pool.asset_pubkey,
                is_signer: false,
                is_writable: false,
            },
            AccountMeta {
                pubkey: *pool_token_account.key,
                is_signer: false,
                is_writable: true,
            },
            AccountMeta {
                pubkey: *user_token_account.key,
                is_signer: false,
                is_writable: true,
            },
        ],
        data: transfer_ix_data,
    };

    // Execute the transfer
    invoke(
        &transfer_ix,
        &[
            pool_account.clone(),
            token_mint.clone(),
            pool_token_account.clone(),
            user_token_account.clone(),
        ],
    )?;

    user_deposit.deposited_amount = 0;
    user_deposit.status = DepositStatus::Withdrawn;

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

pub fn get_user_deposit(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<Deposits, ProgramError> {
    let account_iter = &mut accounts.iter();
    let user_account = next_account_info(account_iter)?;

    let query_param = match QueryParam::try_from_slice(instruction_data) {
        Ok(list) => list,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };

    let user_deposit_list = match UserDepositList::try_from_slice(&user_account.data.borrow()) {
        Ok(list) => list,
        Err(_) => UserDepositList {
            deposits: Vec::new(),
        },
    };

    let user_deposit = if let Some(deposit) = user_deposit_list
        .deposits
        .iter()
        .find(|d| d.pool_id == query_param.pool_id)
    {
        deposit
    } else {
        return Err(ProgramError::InvalidAccountData);
    };

    Ok(user_deposit.clone())
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

    for pool_id in pool_list.pools {
        let pool_pubkey = pool_list
            .pool_id_to_pubkey
            .iter()
            .find(|(id, _)| *id == pool_id)
            .map(|(_, pubkey)| pubkey);
        if let Some(pool_pubkey) = pool_pubkey {
            let pool_account = accounts
                .iter()
                .find(|acc| acc.key == pool_pubkey)
                .ok_or(ProgramError::InvalidAccountData)?;

            let pool: Pool = Pool::try_from_slice(&pool_account.data.borrow())
                .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

            pools.push(pool);
        } else {
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(pools)
}

pub fn get_pool_by_id(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<Pool, ProgramError> {
    let account_iter = &mut accounts.iter();

    let pool_id = match u64::try_from_slice(instruction_data) {
        Ok(list) => list,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };

    let pool_list_account = next_account_info(account_iter)?;
    let pool_list = PoolList::try_from_slice(&pool_list_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let pool_pubkey = pool_list
        .pool_id_to_pubkey
        .iter()
        .find(|(id, _)| *id == pool_id)
        .map(|(_, pubkey)| pubkey);

    if let Some(pool_pubkey) = pool_pubkey {
        let pool_account = account_iter
            .find(|account| account.key == pool_pubkey)
            .ok_or(ProgramError::InvalidAccountData)?;

        let pool = Pool::try_from_slice(&pool_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

        Ok(pool)
    } else {
        Err(ProgramError::InvalidArgument)
    }
}

pub fn get_pool_tvl(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<u64, ProgramError> {
    let account_iter = &mut accounts.iter();
    let pool_list_account = next_account_info(account_iter)?;
    let pool_list = PoolList::try_from_slice(&pool_list_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let pool_id = match u64::try_from_slice(instruction_data) {
        Ok(list) => list,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };

    let pool_pubkey = pool_list
        .pool_id_to_pubkey
        .iter()
        .find(|(id, _)| *id == pool_id)
        .map(|(_, pubkey)| pubkey);

    if let Some(pool_pubkey) = pool_pubkey {
        let pool_account = account_iter
            .find(|account| account.key == pool_pubkey)
            .ok_or(ProgramError::InvalidAccountData)?;

        let pool = Pool::try_from_slice(&pool_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;

        Ok(pool.tvl)
    } else {
        Err(ProgramError::InvalidArgument)
    }
}
