use askama_axum::{IntoResponse, Response};
use axum::http::StatusCode;
use redb::{Key, TypeName, Value};
use std::cmp::Ordering;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UuidKey(pub Uuid);

const SIZE_OF_BYTES: usize = size_of::<uuid::Bytes>();
impl Value for UuidKey {
    type SelfType<'a> = UuidKey;
    type AsBytes<'a> = uuid::Bytes;

    fn fixed_width() -> Option<usize> {
        Some(SIZE_OF_BYTES)
    }

    fn from_bytes<'a>(serialized_uuid: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        debug_assert_eq!(serialized_uuid.len(), SIZE_OF_BYTES);
        let serialized_uuid: uuid::Bytes = serialized_uuid[0..SIZE_OF_BYTES].try_into().unwrap();
        let uuid = Uuid::from_bytes(serialized_uuid);
        UuidKey(uuid)
    }

    fn as_bytes<'a, 'b: 'a>(uuid_key: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        *uuid_key.0.as_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new("nonce_guess::UuidValue")
    }
}

impl Key for UuidKey {
    fn compare(uuid_key1: &[u8], uuid_key2: &[u8]) -> Ordering {
        let uuid1 = Uuid::from_bytes(uuid_key1[0..SIZE_OF_BYTES].try_into().unwrap());
        let uuid2 = Uuid::from_bytes(uuid_key2[0..SIZE_OF_BYTES].try_into().unwrap());
        uuid1.cmp(&uuid2)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("failed to generate new {0} uuid after {1} tries")]
    NewUuid(String, u8),
    #[error(transparent)]
    RedbTable(#[from] redb::TableError),
    #[error(transparent)]
    RedbTransaction(#[from] redb::TransactionError),
    #[error(transparent)]
    RedbStorage(#[from] redb::StorageError),
    #[error(transparent)]
    RedbCommit(#[from] redb::CommitError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        error!("{}", self);
        (
            StatusCode::OK,
            [("HX-Retarget", "#flash_message")],
            "Internal server error.",
        )
            .into_response()
    }
}

#[cfg(test)]
mod test {
    use crate::types::UuidKey;
    use redb::{Key, Value};
    use uuid::Uuid;

    #[test]
    fn test_uuidkey_encode_decode() {
        let orig_uuidkey = UuidKey(Uuid::new_v4());
        let encoded_uuidkey = UuidKey::as_bytes(&orig_uuidkey);
        let decoded_uuidkey = UuidKey::from_bytes(&encoded_uuidkey);
        assert_eq!(orig_uuidkey, decoded_uuidkey);
    }

    #[test]
    fn test_uuidkey_compare() {
        let uuid_key1 = UuidKey(Uuid::new_v4());
        let encoded_uuid_key1 = UuidKey::as_bytes(&uuid_key1);
        let uuid_key2 = UuidKey(Uuid::new_v4());
        let encoded_uuid_key2 = UuidKey::as_bytes(&uuid_key2);
        assert_eq!(
            UuidKey::compare(&encoded_uuid_key1, &encoded_uuid_key2),
            uuid_key1.0.cmp(&uuid_key2.0)
        );
    }
}
