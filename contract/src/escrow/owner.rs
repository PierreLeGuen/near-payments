use near_sdk::json_types::U128;

use crate::*;

// Owner functions
// All the calls comes from the multisig wallet, no need to be part of the contract ext functions
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
        };

        self.escrow_transfers.insert(&id, &p);

        // return escrow payment id
        Ok(p.id)
    }

    // pub fn create_ft_escrow_payment(
    //     &mut self,
    //     receiver_id: AccountId,
    //     amount: u128,
    //     token_account: AccountId,
    //     label: String,
    //     is_locked: bool,
    // ) -> Result<CryptoHash, String> {
    //     // check if token account is registered
    //     if self.ft_committed_balance.get(&token_account).is_none() {
    //         self.ft_committed_balance.insert(&token_account, &0);
    //     };

    //     let ft_balance =
    //         ext_nep141_token::ext(token_account).ft_balance_of(env::current_account_id());

    //     // check near balance is sufficient
    //     assert!(
    //         env::account_balance()
    //             >= self.ft_committed_balance.get(&token_account).unwrap() + amount,
    //         "Not enough NEAR balance"
    //     );

    //     // update committed balance
    //     self.near_committed_balance += amount;

    //     // create escrow payment
    //     let mut buf = env::random_seed();
    //     buf.append(&mut self.request_nonce.to_le_bytes().to_vec());

    //     let id: CryptoHash = env::sha256(&buf).as_slice().try_into().unwrap();
    //     let p = EscrowTransfer {
    //         id,
    //         receiver_id,
    //         amount,
    //         label,
    //         is_locked,
    //     };

    //     self.escrow_transfers.insert(&id, &p);

    //     // return escrow payment id
    //     Ok(p.id)
    // }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_create_ft_escrow(
        &self,
        #[callback_result] balance: Result<U128, near_sdk::PromiseError>,
    ) -> MultiSigResponse {
        let a = match balance {
            Ok(b) => b,
            Err(e) => panic!("Error: {:?}", e),
        };
        MultiSigResponse::new(0, FuncResponse::Balance(a))
    }
}
