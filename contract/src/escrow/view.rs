use near_sdk::json_types::Base58CryptoHash;

use crate::{
    common::errors::{self, ContractError},
    *,
};

#[near_bindgen]
impl Contract {
    #[handle_result]
    pub fn get_payments(&self) -> Result<Vec<(CryptoHash, EscrowTransfer)>, String> {
        let mut payments = vec![];
        for (k, v) in self.escrow_transfers.iter() {
            payments.push((k, v));
        }
        Ok(payments)
    }

    #[handle_result]
    pub fn get_payment_by_ud(
        &self,
        payment_id: Base58CryptoHash,
    ) -> Result<EscrowTransfer, ContractError> {
        self.escrow_transfers
            .get(&payment_id.into())
            .ok_or_else(|| ContractError::EscrowTransferNotFound("payment not found".into()))
    }
}
