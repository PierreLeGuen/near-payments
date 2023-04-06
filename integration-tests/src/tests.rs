use anyhow::Ok;
use near_sdk::AccountId;
use near_units::parse_near;
use serde_json::json;
use std::{env, fs};
use workspaces::{Account, Contract};

pub use models::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wasm_arg: &str = &(env::args().nth(1).unwrap());
    let wasm_filepath = fs::canonicalize(env::current_dir()?.join(wasm_arg))?;

    println!("deploying contract to sandbox");
    let worker = workspaces::sandbox().await?;
    let wasm = std::fs::read(wasm_filepath)?;
    let contract = worker.dev_deploy(&wasm).await?;

    println!("creating accounts");
    // create accounts
    let account = worker.dev_create_account().await?;
    let alice = account
        .create_subaccount("alice")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;

    println!("initializing contract");
    init_multsig_contract(
        contract.as_account(),
        vec![MultisigMember::Account {
            account_id: near_sdk::AccountId::new_unchecked("alice".to_string()),
        }],
    )
    .await?;

    println!("running tests");
    // begin tests
    test_default_message(&alice, &contract).await?;
    test_changes_message(&alice, &contract).await?;
    Ok(())
}

async fn init_multsig_contract(
    multisig_acc_id: &Account,
    members: Vec<MultisigMember>,
) -> anyhow::Result<()> {
    let args = json!({
        "members": members,
        "num_confirmations": 1,
    });

    dbg!(args.clone());

    multisig_acc_id
        .call(multisig_acc_id.id(), "new")
        .args_json(args)
        .transact()
        .await?
        .into_result()?;

    Ok(())
}

async fn test_default_message(user: &Account, contract: &Contract) -> anyhow::Result<()> {
    let message: String = user
        .call(contract.id(), "get_greeting")
        .args_json(json!({}))
        .transact()
        .await?
        .json()?;

    assert_eq!(message, "Hello".to_string());
    println!("      Passed ✅ gets default message");
    Ok(())
}

async fn test_changes_message(user: &Account, contract: &Contract) -> anyhow::Result<()> {
    user.call(contract.id(), "set_greeting")
        .args_json(json!({"message": "Howdy"}))
        .transact()
        .await?
        .into_result()?;

    let message: String = user
        .call(contract.id(), "get_greeting")
        .args_json(json!({}))
        .transact()
        .await?
        .json()?;

    assert_eq!(message, "Howdy".to_string());
    println!("      Passed ✅ changes message");
    Ok(())
}
