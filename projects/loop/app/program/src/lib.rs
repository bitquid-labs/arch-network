use agent::create_agent;
use arch_program::{
    account::AccountInfo, entrypoint, msg, program::next_account_info, program_error::ProgramError,
    pubkey::Pubkey,
};
use mint::{initialize_mint, mint_tokens, MintInput};
use token_account::initialize_balance_account;
use transfer::{transfer_tokens, TransferInput};
pub mod agent;
pub mod errors;
pub mod mint;
pub mod token_account;
pub mod transfer;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();

    match instruction_data[0] {
        0 => {
            msg!("Creating Agent....");

            if accounts.len() != 1 {
                return Err(ProgramError::Custom(502));
            }

            create_agent(program_id, accounts, &instruction_data[1..])?;
        }
        1 => {
            if accounts.len() != 3 {
                return Err(ProgramError::Custom(502));
            }

            let owner_account = next_account_info(account_iter)?;

            let mint_account = next_account_info(account_iter)?;

            let balance_account = next_account_info(account_iter)?;

            initialize_balance_account(owner_account, mint_account, balance_account, program_id)?;
        }
        2 => {
            if accounts.len() != 3 {
                return Err(ProgramError::Custom(502));
            }

            let mint_account = next_account_info(account_iter)?;

            let balance_account = next_account_info(account_iter)?;

            let owner_account = next_account_info(account_iter)?;

            let mint_input: MintInput = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;

            mint_tokens(
                balance_account,
                mint_account,
                owner_account,
                program_id,
                mint_input,
            )?;
        }
        3 => {
            if accounts.len() != 4 {
                return Err(ProgramError::Custom(502));
            }

            let owner_account = next_account_info(account_iter)?;

            let mint_account = next_account_info(account_iter)?;

            let sender_account = next_account_info(account_iter)?;

            let receiver_account = next_account_info(account_iter)?;

            let transfer_input: TransferInput = borsh::from_slice(&instruction_data[1..])
                .map_err(|_e| ProgramError::InvalidArgument)?;

            transfer_tokens(
                owner_account,
                mint_account,
                sender_account,
                receiver_account,
                program_id,
                transfer_input,
            )?;
        }
        _ => {
            msg!("Invalid argument provided !");
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(())
}
