use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, serde_json};
use near_sdk::{
    serde::{Deserialize, Serialize},
    AccountId, PublicKey,
};

/// Represents member of the multsig: either account or access key to given account.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde", untagged)]
pub enum MultisigMember {
    AccessKey { public_key: PublicKey },
    Account { account_id: AccountId },
}

impl ToString for MultisigMember {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| env::panic_str("Failed to serialize"))
    }
}
