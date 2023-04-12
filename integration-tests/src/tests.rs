use anyhow::{Ok, Result};
use near_sdk::{
    json_types::{Base58CryptoHash, U128},
    AccountId, CryptoHash, Gas, ONE_NEAR, ONE_YOCTO,
};

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
            "num_confirmations": num_confirmations,
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
    ) -> Result<Option<MultiSigResponse>> {
        let ret = from
            .call(self.contract.id(), "add_request_and_confirm")
            .args_json(json!({ "request": request }))
            .gas(300 * Gas::ONE_TERA.0)
            .transact()
            .await?
            .into_result()?;

        let r = if !ret.raw_bytes().unwrap().is_empty() {
            let q: MultiSigResponse = ret.json().unwrap();
            Some(q)
        } else {
            None
        };
        Ok(r)
    }

    async fn get_payments(
        &self,
        caller: &Account,
        payment_id: Base58CryptoHash,
    ) -> Result<Vec<(CryptoHash, EscrowTransfer)>> {
        let v: Vec<(CryptoHash, EscrowTransfer)> = caller
            .call(self.contract.id(), "get_payments")
            .transact()
            .await?
            .json()?;

        Ok(v)
    }

    async fn claim_payment(&self, caller: &Account, payment_id: Base58CryptoHash) -> Result<()> {
        caller
            .call(self.contract.id(), "claim_payment")
            .args_json(json!({ "payment_id": payment_id }))
            .gas(300 * Gas::ONE_TERA.0)
            .deposit(ONE_YOCTO)
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
}

// TODO(pierre): Switch to tokio testing API. Current solution doesn't scale.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wasm_arg: &str = &(env::args().nth(1).unwrap());
    let wasm_filepath = fs::canonicalize(env::current_dir()?.join(wasm_arg))?;

    println!("deploying contract to sandbox");
    let worker = workspaces::sandbox().await?;
    let wasm = fs::read(wasm_filepath)?;
    let contract = worker.dev_deploy(&wasm).await?;

    let contract_wrapper = ContractWrapper::new(contract.clone());

    println!("deploying wrap near contract to sandbox and initializing it");
    let wrap_near_contract = worker.dev_deploy(&fs::read("./res/w_near.wasm")?).await?;
    wrap_near_contract.call("new").transact().await?.unwrap();

    println!("creating accounts");
    // create accounts
    let alice = worker.dev_create_account().await?;
    let bob = worker.dev_create_account().await?;

    println!("transferring wrap near to multisig wallet");
    alice
        .call(wrap_near_contract.id(), "near_deposit")
        .deposit(50 * ONE_NEAR)
        .transact()
        .await?
        .unwrap();

    alice
        .call(wrap_near_contract.id(), "storage_deposit")
        .args_json(json!({ "account_id": contract.id() }))
        .deposit(parse_near!("0.00125"))
        .transact()
        .await?
        .unwrap();

    alice
        .call(wrap_near_contract.id(), "ft_transfer")
        .args_json(json!({ "receiver_id": contract.id(), "amount": (40 * ONE_NEAR).to_string() }))
        .deposit(ONE_YOCTO)
        .transact()
        .await?
        .unwrap();

    let b: U128 = bob
        .call(wrap_near_contract.id(), "ft_balance_of")
        .args_json(json!({
            "account_id": contract.id()
        }))
        .transact()
        .await?
        .unwrap()
        .json()?;

    println!("initializing contract");
    contract_wrapper
        .init(
            vec![MultisigMember::Account {
                account_id: workspace_acc_id_to_sdk_id(&alice),
            }],
            1,
        )
        .await?;

    println!("running tests");

    // begin tests
    // test_transfer(&contract_wrapper, &alice, &bob).await?;
    // test_escrow_transfer(&contract_wrapper, &alice, &bob).await?;
    // test_escrow_transfer_above_account_balance_should_panic(&contract_wrapper, &alice, &bob)
    //     .await?;
    test_ft_escrow_transfer(
        &contract_wrapper,
        wrap_near_contract.as_account(),
        &alice,
        &bob,
    )
    .await?;

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
        actions: vec![MultiSigRequestAction::NearEscrowTransfer {
            receiver_id: workspace_acc_id_to_sdk_id(to),
            amount: ONE_NEAR.into(),
            label: "test".to_string(),
            is_cancellable: true,
        }],
    };

    let ret = contract
        .add_request_and_confirm(from, request)
        .await?
        .expect("no response");

    let id = match ret.response {
        FuncResponse::EscrowPayment(p) => p,
        _ => panic!("unexpected response"),
    };

    contract.claim_payment(to, id).await?;

    Ok(())
}

async fn test_escrow_transfer_above_account_balance_should_panic(
    contract: &ContractWrapper,
    caller: &Account,
    to: &Account,
) -> Result<()> {
    let request = MultiSigRequest {
        receiver_id: AccountId::new_unchecked(to.id().to_string()),
        actions: vec![MultiSigRequestAction::NearEscrowTransfer {
            receiver_id: workspace_acc_id_to_sdk_id(to),
            amount: (90 * ONE_NEAR).into(),
            label: "test".to_string(),
            is_cancellable: true,
        }],
    };

    let ret = contract
        .add_request_and_confirm(caller, request)
        .await?
        .expect("no response");

    let id = match ret.response {
        FuncResponse::EscrowPayment(p) => p,
        _ => panic!("unexpected response"),
    };

    let request = MultiSigRequest {
        receiver_id: AccountId::new_unchecked(to.id().to_string()),
        actions: vec![MultiSigRequestAction::NearEscrowTransfer {
            receiver_id: workspace_acc_id_to_sdk_id(to),
            amount: (90 * ONE_NEAR).into(),
            label: "test".to_string(),
            is_cancellable: true,
        }],
    };

    let ret = contract
        .add_request_and_confirm(caller, request)
        .await?
        .expect("no response");

    let id = match ret.response {
        FuncResponse::EscrowPayment(p) => p,
        _ => panic!("unexpected response"),
    };

    // contract.claim_payment(to, id).await?;

    Ok(())
}

async fn test_ft_escrow_transfer(
    contract: &ContractWrapper,
    ft_contract: &Account,
    caller: &Account,
    to: &Account,
) -> Result<()> {
    let request = MultiSigRequest {
        receiver_id: workspace_acc_id_to_sdk_id(to),
        actions: vec![MultiSigRequestAction::FTEscrowTransfer {
            receiver_id: workspace_acc_id_to_sdk_id(to),
            amount: (30 * ONE_NEAR).into(),
            label: "test".to_string(),
            is_cancellable: true,
            token_id: workspace_acc_id_to_sdk_id(ft_contract),
        }],
    };

    let ret = contract
        .add_request_and_confirm(caller, request)
        .await?
        .expect("no response");

    let id = match ret.response {
        FuncResponse::EscrowPayment(id) => dbg!(id),
        _ => panic!("unexpected response"),
    };

    // let p = contract.get_payments(caller, id).await?;

    contract.claim_payment(to, id).await?;

    let b: U128 = to
        .call(ft_contract.id(), "ft_balance_of")
        .args_json(json!({
            "account_id": to.id()
        }))
        .transact()
        .await?
        .unwrap()
        .json()?;

    assert_eq!(b.0, 30 * ONE_NEAR);

    Ok(())
}

// Helper function to convert workspaces::AccountId to near_sdk::AccountId
fn workspace_acc_id_to_sdk_id(acc: &workspaces::Account) -> near_sdk::AccountId {
    near_sdk::AccountId::new_unchecked(acc.id().to_string())
}
