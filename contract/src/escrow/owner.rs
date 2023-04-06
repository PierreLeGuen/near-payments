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
        assert!(env::account_balance() >= amount, "Not enough NEAR balance");

        // update committed balance
        self.near_committed_balance += amount;

        // create escrow payment
        let mut buf = env::random_seed();
        buf.append(&mut self.request_nonce.to_le_bytes().to_vec());

        let id = env::sha256(&buf).as_slice().try_into().unwrap();
        let p = EscrowTransfer {
            id,
            receiver_id,
            amount,
            label,
            is_locked,
        };

        // return escrow payment id
        Ok(p.id)
    }
}
