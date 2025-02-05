use crate::types::InternalError;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

/// A players guess for a target block nonce.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Guess {
    pub player: Uuid,
    pub nonce: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum GuessError {
    #[error("player already made a guess for target height: {0}")]
    DuplicateGuess(u32),
    #[error("invalid nonce: {0}")]
    InvalidNonce(String),
    #[error("nonce was guessed by another player: {0}")]
    DuplicateNonce(String),
    #[error("block was already confirmed for target height: {0}")]
    ConfirmedTarget(u32),
    #[error("target does not exist for height: {0}")]
    MissingTarget(u32),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

#[derive(thiserror::Error, Debug)]
pub enum TargetError {
    #[error("new height less than current target height: {0}")]
    InvalidHeight(u32),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

/// Block data from mempool.space.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Block {
    pub id: String,
    pub height: u32,
    pub nonce: u32,
}

#[cfg(test)]
mod test {
    use crate::guess::types::Guess;
    use redb::Value;
    use uuid::Uuid;

    #[test]
    fn test_guess_encode_decode() {
        let orig_guess = Guess {
            player: Uuid::new_v4(),
            nonce: 12345678,
        };
        let encoded_guess = Guess::as_bytes(&orig_guess);
        let decoded_guess = Guess::from_bytes(&encoded_guess);
        assert_eq!(orig_guess, decoded_guess);
    }
}
