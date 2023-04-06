use crate::*;

use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};

#[near_bindgen]
impl Contract {
    pub fn claim_payment(&mut self, payment_id: CryptoHash) -> Promise {
        let p = self
            .escrow_transfers
            .get(&payment_id)
            .expect("Payment not found");

        // assert called by receiver
        assert_eq!(
            env::predecessor_account_id(),
            p.receiver_id,
            "Only receiver can claim payment"
        );

        // transfer NEAR to receiver
        Promise::new(p.receiver_id)
            .transfer(p.amount)
            .then(Self::ext(env::current_account_id()).callback_claim_payment(p.id))
    }

    #[private]
    pub fn callback_claim_payment(&mut self, payment_id: CryptoHash) {
        let p = self
            .escrow_transfers
            .get(&payment_id)
            .expect("Payment not found");

        self.near_committed_balance -= p.amount;

        // remove payment from escrow
        self.escrow_transfers.remove(&payment_id);
    }
}
