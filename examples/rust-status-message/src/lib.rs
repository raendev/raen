use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, near_bindgen, AccountId};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct StatusMessage {
    records: LookupMap<AccountId, String>,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            records: LookupMap::new(b"r"),
        }
    }
}

#[near_bindgen]
impl StatusMessage {
    /// Store a message for current signer account
    pub fn set_status(&mut self, message: String) {
        let account_id = env::signer_account_id();
        self.records.insert(&account_id, &message);
    }

    /// Retreive a message for a given account id
    pub fn get_status(&self, account_id: AccountId) -> Option<String> {
        self.records.get(&account_id)
    }
}
