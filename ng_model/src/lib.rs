use serde::{Deserialize, Serialize};

pub use serde;
pub use serde_json;
pub use serde_with;

/// The target block that players are trying to guess the nonce for.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct Target {
    pub block: u32,
    pub nonce: Option<u32>,
}

/// A players guess for a target block nonce.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Guess {
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

pub fn sort_guesses_by_nonce(guesses: &mut [Guess]) {
    guesses.sort_by_key(|g| g.nonce)
}

pub fn no_duplicate_guess(
    fetched_guesses: &mut [Guess],
    new_guess: &Guess,
) -> Result<bool, Guess> {
    for fetched_guess in fetched_guesses.iter() {
        if (fetched_guess.block.is_none() || fetched_guess.block == new_guess.block)
            && fetched_guess.name == new_guess.name
        {
            return Err(fetched_guess.clone());
        }
    }
    return Ok(true);
}