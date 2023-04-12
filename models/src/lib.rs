use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58CryptoHash, Base64VecU8, U128};
use near_sdk::{env, serde_json, CryptoHash, Promise};
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

pub type RequestId = u32;

/// Permissions for function call access key.
#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FunctionCallPermission {
    pub allowance: Option<U128>,
    pub receiver_id: AccountId,
    pub method_names: Vec<String>,
}

/// Lowest level action that can be performed by the multisig contract.
#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(tag = "type", crate = "near_sdk::serde")]
pub enum MultiSigRequestAction {
    /// Create a new account.
    CreateAccount,
    /// Deploys contract to receiver's account. Can upgrade given contract as well.
    DeployContract { code: Base64VecU8 },
    /// Add new member of the multisig.
    AddMember { member: MultisigMember },
    /// Remove existing member of the multisig.
    DeleteMember { member: MultisigMember },
    /// Adds full access key to another account.
    AddKey {
        public_key: PublicKey,
        #[serde(skip_serializing_if = "Option::is_none")]
        permission: Option<FunctionCallPermission>,
    },
    /// Sets number of confirmations required to authorize requests.
    /// Can not be bundled with any other actions or transactions.
    SetNumConfirmations { num_confirmations: u32 },
    /// Sets number of active requests (unconfirmed requests) per access key
    /// Default is 12 unconfirmed requests at a time
    /// The REQUEST_COOLDOWN for requests is 15min
    /// Worst gas attack a malicious keyholder could do is 12 requests every 15min
    SetActiveRequestsLimit { active_requests_limit: u32 },
    /// Payment options
    /// Transfers given amount to receiver.
    Transfer { amount: U128 },
    /// NEAR Escrow transfer
    NearEscrowTransfer {
        receiver_id: AccountId,
        amount: U128,
        label: String,
        is_cancellable: bool,
    },
    /// FT Escrow transfer
    FTEscrowTransfer {
        receiver_id: AccountId,
        amount: U128,
        token_id: AccountId,
        label: String,
        is_cancellable: bool,
    },
}

/// The request the user makes specifying the receiving account and actions they want to execute (1 tx)
#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MultiSigRequest {
    pub receiver_id: AccountId,
    pub actions: Vec<MultiSigRequestAction>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Clone))]
#[serde(crate = "near_sdk::serde")]
pub enum FuncResponse {
    AddRequest(RequestId),
    Default(bool),
    EscrowPayment(Base58CryptoHash),
    Balance(U128),
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Clone))]
#[serde(crate = "near_sdk::serde")]
pub struct MultiSigResponse {
    pub request_id: RequestId,
    pub response: FuncResponse,
}

impl MultiSigResponse {
    pub fn new(request_id: RequestId, response: FuncResponse) -> Self {
        Self {
            request_id,
            response,
        }
    }
}
