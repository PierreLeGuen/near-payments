use crate::*;

use near_sdk::{Balance, FunctionError};

#[derive(BorshDeserialize, BorshSerialize, Serialize, PartialEq, Debug)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Deserialize))]
#[serde(crate = "near_sdk::serde")]
pub enum ContractError {
    InsufficientDeposit {
        expected: Balance,
        received: Balance,
    },
    EscrowTransferNotFound(String),
    NotAuthorized,
    NearTransferFailed,
}

impl FunctionError for ContractError {
    fn panic(&self) -> ! {
        crate::env::panic_str(
            &serde_json::to_string(self).unwrap_or(format!("serde failed: {self:?}")),
        )
    }
}
