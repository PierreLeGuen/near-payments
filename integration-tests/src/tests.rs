use anyhow::{Ok, Result};
use near_sdk::{AccountId, ONE_NEAR};
use near_units::parse_near;
use serde_json::json;
use std::{env, fs};
use workspaces::{Account, Contract};

pub use models::*;

pub struct ContractWrapper {
    contract: Contract,
}

impl ContractWrapper {
    fn new(contract: Contract) -> Self {
        Self { contract }
    }

    async fn init(&self, members: Vec<MultisigMember>, num_confirmations: u128) -> Result<()> {
        let args = json!({
            "members": members,
            "num_confirmations": 1,
        });

        self.contract
            .call("new")
            .args_json(args)
            .transact()
            .await?
            .into_result()?;

        let m: Vec<MultisigMember> = self.contract.view("get_members").await?.json()?;

        assert!(m == members);
        Ok(())
    }

    async fn add_request_and_confirm(
        &self,
        from: &Account,
        request: MultiSigRequest,
    ) -> Result<()> {
        from.call(self.contract.id(), "add_request_and_confirm")
            .args_json(json!({ "request": request }))
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wasm_arg: &str = &(env::args().nth(1).unwrap());
    let wasm_filepath = fs::canonicalize(env::current_dir()?.join(wasm_arg))?;

    println!("deploying contract to sandbox");
    let worker = workspaces::sandbox().await?;
    let wasm = std::fs::read(wasm_filepath)?;
    let contract = worker.dev_deploy(&wasm).await?;

    let contract = ContractWrapper::new(contract);

    println!("creating accounts");
    // create accounts
    let alice = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    println!("initializing contract");
    contract
        .init(
            vec![MultisigMember::Account {
                account_id: workspace_acc_id_to_sdk_id(alice.id()),
            }],
            1,
        )
        .await?;

    println!("running tests");
    // begin tests
    test_transfer(&contract, &alice, &bob).await?;
    test_escrow_transfer(&contract, &alice, &bob).await?;

    Ok(())
}

async fn test_transfer(contract: &ContractWrapper, from: &Account, to: &Account) -> Result<()> {
    let request = MultiSigRequest {
        receiver_id: AccountId::new_unchecked(to.id().to_string()),
        actions: vec![MultiSigRequestAction::Transfer {
            amount: ONE_NEAR.into(),
        }],
    };

    let to_before = to.view_account().await?;

    contract.add_request_and_confirm(from, request).await?;

    let to_after = to.view_account().await?;

    assert_eq!(to_after.balance - to_before.balance, ONE_NEAR);

    Ok(())
}

async fn test_escrow_transfer(
    contract: &ContractWrapper,
    from: &Account,
    to: &Account,
) -> Result<()> {
    let request = MultiSigRequest {
        receiver_id: AccountId::new_unchecked(to.id().to_string()),
        actions: vec![MultiSigRequestAction::EscrowTransfer {
            receiver_id: workspace_acc_id_to_sdk_id(to.id()),
            amount: ONE_NEAR.into(),
            label: "test".to_string(),
            is_cancellable: true,
        }],
    };

    let to_before = to.view_account().await?;

    contract.add_request_and_confirm(from, request).await?;

    let to_after = to.view_account().await?;

    assert_eq!(to_after.balance - to_before.balance, ONE_NEAR);

    Ok(())
}

// Helper function to convert workspaces::AccountId to near_sdk::AccountId
fn workspace_acc_id_to_sdk_id(acc_id: &workspaces::AccountId) -> near_sdk::AccountId {
    near_sdk::AccountId::new_unchecked(acc_id.to_string())
}
