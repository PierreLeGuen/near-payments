use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

use near_sdk::{
    serde::{Deserialize, Serialize},
    AccountId, CryptoHash,
};

pub mod owner;
pub mod receiver;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Clone))]
#[serde(crate = "near_sdk::serde")]
pub struct EscrowTransfer {
    id: CryptoHash,
    receiver_id: AccountId,
    amount: u128,
    label: String,
    is_locked: bool,
}
