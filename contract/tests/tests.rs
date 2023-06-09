#[cfg(test)]
mod tests {
    use anyhow::{Ok, Result};
    use near_sdk::{
        json_types::{Base58CryptoHash, U128},
        AccountId, CryptoHash, Gas, ONE_NEAR, ONE_YOCTO,
    };

    use near_units::parse_near;
    use serde_json::json;
    use std::{fs, sync::Once};
    use tracing::info;
    use workspaces::{Account, Contract};

    pub use models::*;

    pub struct ContractWrapper {
        contract: Contract,
    }

    static log_int: Once = Once::new();

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
        ) -> Result<Vec<(CryptoHash, EscrowTransfer)>> {
            let v: Vec<(CryptoHash, EscrowTransfer)> = caller
                .call(self.contract.id(), "get_payments")
                .transact()
                .await?
                .json()?;

            Ok(v)
        }

        async fn claim_payment(
            &self,
            caller: &Account,
            payment_id: Base58CryptoHash,
        ) -> Result<()> {
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

    async fn init() -> Result<(ContractWrapper, Contract, Account, Account)> {
        log_int.call_once(|| {
            // construct a subscriber that prints formatted traces to stdout
            let subscriber = tracing_subscriber::FmtSubscriber::new();
            // use that subscriber to process traces emitted after this point
            tracing::subscriber::set_global_default(subscriber).unwrap();
        });

        let worker = workspaces::sandbox().await?;

        info!("creating accounts");
        let alice = worker.dev_create_account().await?;
        let bob = worker.dev_create_account().await?;

        info!("deploying contracts");
        let payments_contract = worker
            .dev_deploy(&fs::read(
                "../target/wasm32-unknown-unknown/release/near_payments.wasm",
            )?)
            .await?;
        let ft_contract = worker.dev_deploy(&fs::read("./tests/w_near.wasm")?).await?;
        ft_contract.call("new").transact().await?.unwrap();

        let contract_wrapper = ContractWrapper::new(payments_contract.clone());

        info!("transferring wrap near to multisig wallet");
        alice
            .call(ft_contract.id(), "near_deposit")
            .deposit(50 * ONE_NEAR)
            .transact()
            .await?
            .unwrap();

        alice
            .call(ft_contract.id(), "storage_deposit")
            .args_json(json!({ "account_id": payments_contract.id() }))
            .deposit(parse_near!("0.00125"))
            .transact()
            .await?
            .unwrap();

        alice
            .call(ft_contract.id(), "ft_transfer")
            .args_json(
                json!({ "receiver_id": payments_contract.id(), "amount": (40 * ONE_NEAR).to_string() }),
            )
            .deposit(ONE_YOCTO)
            .transact()
            .await?
            .unwrap();

        info!("initializing contract");
        contract_wrapper
            .init(
                vec![MultisigMember::Account {
                    account_id: workspace_acc_id_to_sdk_id(&alice),
                }],
                1,
            )
            .await?;

        Ok((contract_wrapper, ft_contract, alice, bob))
    }

    #[tokio::test]
    async fn test_transfer() -> Result<()> {
        let (contract_wrapper, _, caller, to) = init().await?;

        let request = MultiSigRequest {
            receiver_id: AccountId::new_unchecked(to.id().to_string()),
            actions: vec![MultiSigRequestAction::Transfer {
                amount: ONE_NEAR.into(),
            }],
        };

        contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_escrow_transfer() -> Result<()> {
        let (contract_wrapper, _, caller, to) = init().await?;

        let request = MultiSigRequest {
            receiver_id: AccountId::new_unchecked(to.id().to_string()),
            actions: vec![MultiSigRequestAction::NearEscrowTransfer {
                receiver_id: workspace_acc_id_to_sdk_id(&to),
                amount: ONE_NEAR.into(),
                label: "test".to_string(),
                is_cancellable: true,
            }],
        };

        let ret = contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await?
            .expect("no response");

        let id = match ret.response {
            FuncResponse::EscrowPayment(p) => p,
            _ => panic!("unexpected response"),
        };

        contract_wrapper.claim_payment(&to, id).await?;

        Ok(())
    }

    #[tokio::test]
    #[should_panic = "Not enough NEAR balance"]
    async fn test_escrow_transfer_above_account_balance() {
        let (contract_wrapper, _, caller, to) = init().await.unwrap();

        let request = MultiSigRequest {
            receiver_id: workspace_acc_id_to_sdk_id(&to),
            actions: vec![MultiSigRequestAction::NearEscrowTransfer {
                receiver_id: workspace_acc_id_to_sdk_id(&to),
                amount: (90 * ONE_NEAR).into(),
                label: "test".to_string(),
                is_cancellable: true,
            }],
        };

        let ret = contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await
            .unwrap()
            .expect("no response");

        let _id = match ret.response {
            FuncResponse::EscrowPayment(p) => p,
            _ => panic!("unexpected response"),
        };

        let request = MultiSigRequest {
            receiver_id: workspace_acc_id_to_sdk_id(&to),
            actions: vec![MultiSigRequestAction::NearEscrowTransfer {
                receiver_id: workspace_acc_id_to_sdk_id(&to),
                amount: (90 * ONE_NEAR).into(),
                label: "test".to_string(),
                is_cancellable: true,
            }],
        };

        let ret = contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await
            .unwrap()
            .expect("no response");

        let _id = match ret.response {
            FuncResponse::EscrowPayment(p) => p,
            _ => panic!("unexpected response"),
        };
    }

    #[tokio::test]
    async fn test_ft_escrow_transfer() -> Result<()> {
        let (contract_wrapper, ft_contract, caller, to) = init().await?;

        let request = MultiSigRequest {
            receiver_id: workspace_acc_id_to_sdk_id(&to),
            actions: vec![MultiSigRequestAction::FTEscrowTransfer {
                receiver_id: workspace_acc_id_to_sdk_id(&to),
                amount: (30 * ONE_NEAR).into(),
                label: "test".to_string(),
                is_cancellable: true,
                token_id: workspace_acc_id_to_sdk_id(ft_contract.as_account()),
            }],
        };

        let ret = contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await?
            .expect("no response");

        let payment_id = match ret.response {
            FuncResponse::EscrowPayment(id) => id,
            _ => panic!("unexpected response"),
        };

        // let p = contract.get_payments(caller, id).await?;

        contract_wrapper.claim_payment(&to, payment_id).await?;

        let balance: U128 = to
            .call(ft_contract.id(), "ft_balance_of")
            .args_json(json!({
                "account_id": to.id()
            }))
            .transact()
            .await?
            .unwrap()
            .json()?;

        assert_eq!(balance.0, 30 * ONE_NEAR);

        Ok(())
    }

    #[tokio::test]
    #[should_panic = "Not enough"]
    async fn test_ft_escrow_transfer_above_balance() {
        let (contract_wrapper, ft_contract, caller, to) = init().await.unwrap();

        let request = MultiSigRequest {
            receiver_id: workspace_acc_id_to_sdk_id(&to),
            actions: vec![MultiSigRequestAction::FTEscrowTransfer {
                receiver_id: workspace_acc_id_to_sdk_id(&to),
                amount: (30 * ONE_NEAR).into(),
                label: "test".to_string(),
                is_cancellable: true,
                token_id: workspace_acc_id_to_sdk_id(ft_contract.as_account()),
            }],
        };

        contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await
            .unwrap();

        let request = MultiSigRequest {
            receiver_id: workspace_acc_id_to_sdk_id(&to),
            actions: vec![MultiSigRequestAction::FTEscrowTransfer {
                receiver_id: workspace_acc_id_to_sdk_id(&to),
                amount: (30 * ONE_NEAR).into(),
                label: "test".to_string(),
                is_cancellable: true,
                token_id: workspace_acc_id_to_sdk_id(ft_contract.as_account()),
            }],
        };

        contract_wrapper
            .add_request_and_confirm(&caller, request)
            .await
            .unwrap();
    }

    // Helper function to convert workspaces::AccountId to near_sdk::AccountId
    fn workspace_acc_id_to_sdk_id(acc: &workspaces::Account) -> near_sdk::AccountId {
        near_sdk::AccountId::new_unchecked(acc.id().to_string())
    }
}
