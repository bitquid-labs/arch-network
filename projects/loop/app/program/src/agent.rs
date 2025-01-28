use arch_program::{account::AccountInfo, program::next_account_info, program_error::ProgramError, pubkey::Pubkey};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{initialize_mint, mint::InitializeMintInput};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Agent {
    name: String,
    cid: String,
    token_mint: Pubkey,
    ticker: String,
    description: String,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AgentParam {
    owner: [u8; 32],
    name: String,
    cid: String,
    description: String,
    ticker: String,
    supply: u64,
    decimals: u8,
}

pub fn create_agent(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let agent_account = next_account_info(account_info_iter)?;
    let agent_mint_account = next_account_info(account_info_iter)?;

    let agent_param = match AgentParam::try_from_slice(instruction_data) {
        Ok(agent_param) => agent_param,
        Err(_) => return Err(ProgramError::InvalidInstructionData),
    };
    let mint_input = InitializeMintInput::new(agent_param.owner, agent_param.supply, agent_param.ticker.clone(), agent_param.decimals);
    let token_mint = initialize_mint(agent_mint_account, program_id, mint_input)?;

    let agent = Agent {
        name: agent_param.name,
        cid: agent_param.cid,
        token_mint,
        ticker: agent_param.ticker,
        description: agent_param.description,
    };

    agent.serialize(&mut &mut agent_account.data.borrow_mut()[..]).map_err(|_e| ProgramError::AccountBorrowFailed)?;

    Ok(())
}
