use super::Backend;
use askama_axum::{IntoResponse, Response};
use axum::http::StatusCode;
use axum_login::AuthUser;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;
use tracing::{error, info};
use uuid::Uuid;

/// The players information
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub uuid: Uuid,
    pub name: String,
    pub password_hash: String,
    pub permissions: HashSet<Permission>,
    pub roles: HashSet<Uuid>,
}

impl AuthUser for Player {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.uuid
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

impl Value for Player {
    type SelfType<'a> = Player;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(serialized_player: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        ciborium::from_reader(serialized_player).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let mut serialized_player = Vec::<u8>::new();
        ciborium::into_writer(value, &mut serialized_player).expect("Failed to serialize player");
        serialized_player
    }

    fn type_name() -> TypeName {
        TypeName::new("nonce_guess::Player")
    }
}

// Permissions that can be granted to a player.
#[serde_with::skip_serializing_none]
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    strum_macros::Display,
    strum_macros::EnumString,
)]
#[serde(deny_unknown_fields)]
pub enum Permission {
    /// Assign a player to the admin role.
    AssignAdm,
    /// Assign a player to the moderator role.
    AssignMod,
    /// Change the target block height.
    ChangeTargetBlock,
}

/// Role (collection of permissions) that can be granted to a player
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub uuid: Uuid,
    pub name: String,
    pub permissions: HashSet<Permission>,
}

impl Value for Role {
    type SelfType<'a> = Role;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(serialized_role: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        ciborium::from_reader(serialized_role).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let mut serialized_role = Vec::<u8>::new();
        ciborium::into_writer(value, &mut serialized_role).expect("Failed to serialize role");
        serialized_role
    }

    fn type_name() -> TypeName {
        TypeName::new("nonce_guess::Role")
    }
}

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
pub enum RegisterError {
    #[error("invalid user name")]
    InvalidName,
    #[error("invalid password")]
    InvalidPassword,
    #[error("password not confirmed")]
    UnconfirmedPassword,
    #[error("user already registered: {0}")]
    UserAlreadyRegistered(String),
    #[error("failed authentication for name: {0}")]
    Authentication(String),
    #[error(transparent)]
    Internal(#[from] axum_login::Error<Backend>),
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> Response {
        match self {
            RegisterError::InvalidName => {
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Name must be 3-20 characters and only include upper or lowercase A-Z, 0-9, and underscore.",
                )
                    .into_response()
            }
            RegisterError::InvalidPassword => {
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Password must be 8-20 characters and include at least one uppercase, one lowercase, one number, and one special character [ @ $ ! % * ? & # ^ _ ].",
                )
                    .into_response()
            }
            RegisterError::UnconfirmedPassword => {
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Password and confirmation do not match.",
                )
                    .into_response()
            }
            RegisterError::UserAlreadyRegistered(user) => {
                info!("user already registered: {}", user);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Name is already registered.",
                )
                    .into_response()
            }
            RegisterError::Authentication(name) => {
                info!("failed authentication for: {}", name);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Failed authentication.",
                )
                    .into_response()
            }
            RegisterError::Internal(e) => {
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

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("failed authentication for name: {0}")]
    Authentication(String),
    #[error(transparent)]
    Internal(#[from] axum_login::Error<Backend>),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            LoginError::Authentication(name) => {
                info!("failed authentication for: {}", name);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    format!("Failed authentication for: {}", name),
                )
                    .into_response()
            }
            LoginError::Internal(_) => {
                error!("{}", self);
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

#[cfg(test)]
mod test {
    use crate::web::auth::types::{Permission, Role, UuidKey};
    use redb::{Key, Value};
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn test_role_encode_decode() {
        let orig_role = Role {
            uuid: Uuid::new_v4(),
            name: "test".to_string(),
            permissions: HashSet::from([Permission::AssignAdm]),
        };
        let encoded_role = Role::as_bytes(&orig_role);
        let decoded_role = Role::from_bytes(&encoded_role);
        assert_eq!(orig_role, decoded_role);
    }

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
