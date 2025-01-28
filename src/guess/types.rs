use crate::types::InternalError;
use askama_axum::{IntoResponse, Response};
use axum::http::StatusCode;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tracing::{error, info};
use uuid::Uuid;

/// A players guess for a target block nonce.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Guess {
    pub player: Uuid,
    pub nonce: u32,
}

impl Value for Guess {
    type SelfType<'a> = Guess;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(serialized_guess: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        ciborium::from_reader(serialized_guess).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let mut serialized_guess = Vec::<u8>::new();
        ciborium::into_writer(value, &mut serialized_guess).expect("Failed to serialize guess");
        serialized_guess
    }

    fn type_name() -> TypeName {
        TypeName::new("nonce_guess::Guess")
    }
}

impl Key for Guess {
    fn compare(uuid_key1: &[u8], uuid_key2: &[u8]) -> Ordering {
        let guess1 = Guess::from_bytes(uuid_key1);
        let guess2 = Guess::from_bytes(uuid_key2);
        guess1.nonce.cmp(&guess2.nonce)
    }
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

impl IntoResponse for GuessError {
    fn into_response(self) -> Response {
        match self {
            GuessError::DuplicateGuess(height) => {
                info!("player already made a guess for target height: {}", height);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    format!("You already made a guess for target height: {}", height),
                )
                    .into_response()
            }
            GuessError::InvalidNonce(_) => (
                StatusCode::OK,
                [("HX-Retarget", "#flash_message")],
                "Invalid nonce.",
            )
                .into_response(),
            GuessError::DuplicateNonce(_) => (
                StatusCode::OK,
                [("HX-Retarget", "#flash_message")],
                "Guess made by another player.",
            )
                .into_response(),
            GuessError::ConfirmedTarget(height) => {
                info!("block was already confirmed for target height: {}", height);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Block already confirmed.",
                )
                    .into_response()
            }
            GuessError::MissingTarget(height) => {
                info!("target does not exist for height: {}", height);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    format!("No target for height: {}", height),
                )
                    .into_response()
            }
            GuessError::Internal(e) => {
                error!("{}", e);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Internal server error.",
                )
                    .into_response()
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TargetError {
    #[error("new height less than current target height: {0}")]
    InvalidHeight(u32),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

impl IntoResponse for TargetError {
    fn into_response(self) -> Response {
        match self {
            TargetError::InvalidHeight(height) => {
                info!(
                    "new height less than or equal to current target height: {}",
                    height
                );
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "New height must be greater than current target height.",
                )
                    .into_response()
            }
            TargetError::Internal(e) => {
                error!("{}", e);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Internal server error.",
                )
                    .into_response()
            }
        }
    }
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
