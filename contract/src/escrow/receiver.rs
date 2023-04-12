use near_sdk::{json_types::Base58CryptoHash, near_bindgen, ONE_YOCTO};

use crate::{
    common::{errors::ContractError, primitives::check_deposit},
    *,
};

use near_units::parse_near;

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
    #[payable]
    pub fn claim_payment(
        &mut self,
        payment_id: Base58CryptoHash,
    ) -> Result<Promise, ContractError> {
        check_deposit(ONE_YOCTO)?;

        let p = self
            .escrow_transfers
            .get(&payment_id.into())
            .ok_or_else(|| ContractError::EscrowTransferNotFound("in claim payment".into()))?;

        // assert called by receiver
        if env::predecessor_account_id() != p.receiver_id {
            return Err(ContractError::NotAuthorized);
        }

        if p.token_account.is_some() {
            // update committed balance
            let curr = self
                .ft_committed_balance
                .get(&p.token_account.clone().unwrap())
                .unwrap();
            let new = curr - p.amount;

            self.ft_committed_balance
                .insert(&p.token_account.clone().unwrap(), &new);

            // remove payment from escrow
            self.escrow_transfers.remove(&payment_id.into());

            // transfer FT to receiver
            Ok(ext_nep141_token::ext(p.token_account.clone().unwrap())
                .with_attached_deposit(parse_near!("0.00125"))
                .storage_deposit(p.receiver_id.clone(), Some(true))
                .then(
                    ext_nep141_token::ext(p.token_account.clone().unwrap())
                        .with_attached_deposit(ONE_YOCTO)
                        .ft_transfer(p.receiver_id.clone(), p.amount.into(), None),
                ))
        } else {
            self.near_committed_balance -= p.amount;

            // remove payment from escrow
            self.escrow_transfers.remove(&payment_id.into());

            // transfer NEAR to receiver
            Ok(Promise::new(p.receiver_id).transfer(p.amount))
        }
    }
}
