use near_sdk::{env, Balance};

use super::errors::ContractError;

pub fn check_deposit(deposit_needed: Balance) -> Result<(), ContractError> {
    if env::attached_deposit() >= deposit_needed {
        Ok(())
    } else {
        Err(ContractError::InsufficientDeposit {
            expected: deposit_needed,
            received: env::attached_deposit(),
        })
    }
}
