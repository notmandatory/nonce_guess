use serde::{Deserialize, Serialize};

pub use serde;
use uuid::Uuid;
use webauthn_rs::prelude::Passkey;

/// The players login information
#[derive(Debug, Clone)]
pub struct Player {
    pub(crate) uuid: Uuid,
    pub(crate) name: String,
    pub(crate) passkeys: Vec<Passkey>,
}

/// The target block that players are trying to guess the nonce for.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct Target {
    pub block: u32,
    pub nonce: Option<u32>,
}

/// A players guess for a target block nonce.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct Guess {
    pub uuid: Uuid,
    pub block: Option<u32>,
    pub name: String,
    pub nonce: u32,
}

/// Block data from mempool.space.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Block {
    pub id: String,
    pub height: u32,
    pub nonce: u32,
}

pub fn sort_guesses_by_target_diff(guesses: &mut [Guess], target_nonce: u32) {
    guesses.sort_by(|a, b| {
        let target_a = target_nonce.abs_diff(a.nonce);
        let target_b = target_nonce.abs_diff(b.nonce);
        target_a.cmp(&target_b)
    })
}

// pub fn sort_guesses_by_nonce(guesses: &mut [Guess]) {
//     guesses.sort_by_key(|g| g.nonce)
// }
