use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen,
    serde::{Deserialize, Serialize},
    witgen,
};

mod views;
/// A message that contains some text
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
#[witgen]
pub struct Message {
    /// Inner string value
    /// @pattern ^TEXT:
    text: String,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    message: Message,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            message: Message {
                text: "initial text".into(),
            },
        }
    }
}

#[near_bindgen]
impl Contract {
    /// A change call to set the message
    pub fn set_message(&mut self, message: Message) {
        self.message = message;
    }
}
