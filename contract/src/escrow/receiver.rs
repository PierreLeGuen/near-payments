use near_sdk::{near_bindgen, ONE_YOCTO};

use crate::{
    common::{errors::ContractError, primitives::check_deposit},
    *,
};

#[near_bindgen]
impl Contract {
    #[handle_result]
    #[payable]
    pub fn claim_payment(&mut self, payment_id: CryptoHash) -> Result<Promise, ContractError> {
        check_deposit(ONE_YOCTO)?;
        let p = self
            .escrow_transfers
            .get(&payment_id)
            .ok_or(ContractError::EscrowTransferNotFound)?;

        // assert called by receiver
        if env::predecessor_account_id() != p.receiver_id {
            return Err(ContractError::NotAuthorized);
        }

        // transfer NEAR to receiver
        Ok(Promise::new(p.receiver_id)
            .transfer(p.amount)
            .then(Self::ext(env::current_account_id()).callback_claim_payment(p.id)))
    }

    #[handle_result]
    #[private]
    pub fn callback_claim_payment(
        &mut self,
        payment_id: CryptoHash,
        #[callback_result] promise_res: Result<(), near_sdk::PromiseError>,
    ) -> Result<(), ContractError> {
        if promise_res.is_err() {
            return Err(ContractError::NearTransferFailed);
        }

        let p = self
            .escrow_transfers
            .get(&payment_id)
            .ok_or(ContractError::EscrowTransferNotFound)?;

        self.near_committed_balance -= p.amount;

        // remove payment from escrow
        self.escrow_transfers.remove(&payment_id);

        Ok(())
    }
}
