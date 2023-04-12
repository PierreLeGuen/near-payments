use near_sdk::json_types::U128;

use crate::EscrowTransfer;
use crate::*;

///
/// Owner functions
///

impl Contract {
    pub fn create_near_escrow_payment(
        &mut self,
        receiver_id: AccountId,
        amount: u128,
        label: String,
        is_locked: bool,
    ) -> Result<CryptoHash, String> {
        // check near balance is sufficient
        assert!(
            env::account_balance() >= self.near_committed_balance + amount,
            "Not enough NEAR balance"
        );

        // update committed balance
        self.near_committed_balance += amount;

        // create escrow payment
        let mut buf = env::random_seed();
        buf.append(&mut self.request_nonce.to_le_bytes().to_vec());

        let id: CryptoHash = env::sha256(&buf).as_slice().try_into().unwrap();
        let p = EscrowTransfer {
            id,
            receiver_id,
            amount,
            label,
            is_locked,
            token_account: None,
        };

        self.escrow_transfers.insert(&id, &p);

        // return escrow payment id
        Ok(p.id)
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_create_ft_escrow(
        &mut self,
        receiver_id: AccountId,
        amount: u128,
        label: String,
        is_locked: bool,
        token_account: AccountId,
        #[callback_result] balance: Result<U128, near_sdk::PromiseError>,
    ) -> MultiSigResponse {
        let balance: u128 = match balance {
            Ok(b) => b.into(),
            Err(e) => env::panic_str(&format!("Error from ft_balance_of: {:?}", e)),
        };

        // assert token account is registered
        if self.ft_committed_balance.get(&token_account).is_none() {
            self.ft_committed_balance.insert(&token_account, &0);
        };

        let mut committed_balance = self.ft_committed_balance.get(&token_account).unwrap();

        // check ft balance is sufficient
        assert!(
            balance >= committed_balance + amount,
            "Not enough {} balance, current balance: {}, committed balance: {}",
            token_account,
            balance,
            committed_balance
        );

        // update committed balance
        committed_balance += amount;
        self.ft_committed_balance
            .insert(&token_account, &committed_balance);

        // create escrow payment
        let mut buf = env::random_seed();
        buf.append(&mut self.request_nonce.to_le_bytes().to_vec());

        let id: CryptoHash = env::sha256(&buf).as_slice().try_into().unwrap();
        let p = EscrowTransfer {
            id,
            receiver_id,
            amount,
            label,
            is_locked,
            token_account: Some(token_account),
        };

        self.escrow_transfers.insert(&id, &p);

        // return escrow payment id
        MultiSigResponse::new(0, FuncResponse::EscrowPayment(id.into()))
    }
}
