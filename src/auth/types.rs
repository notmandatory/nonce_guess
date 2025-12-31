use super::backend::AuthBackend;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

// A helper functions that return the current date time.
pub fn datetime_now() -> DateTime<Utc> {
    Utc::now()
}

/// The players information
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub uuid: Uuid,
    pub name: String,
    pub password_hash: String,
    pub permissions: HashSet<Permission>,
    pub roles: HashSet<Uuid>,
    #[serde(default = "datetime_now")]
    pub last_login: DateTime<Utc>,
    #[serde(default = "datetime_now")]
    pub updated: DateTime<Utc>,
    #[serde(default = "datetime_now")]
    pub created: DateTime<Utc>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            uuid: Default::default(),
            name: "".to_string(),
            password_hash: "".to_string(),
            permissions: Default::default(),
            roles: Default::default(),
            last_login: datetime_now(),
            updated: datetime_now(),
            created: datetime_now(),
        }
    }
}

// Permissions that can be granted to a player.
#[serde_with::skip_serializing_none]
#[derive(Ord, PartialOrd, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub enum Permission {
    /// Assign a player to the admin role.
    AssignAdm,
    /// Assign a player to the moderator role.
    AssignMod,
    /// Change the target block height.
    ChangeTarget,
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
    Internal(#[from] axum_login::Error<AuthBackend>),
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("failed authentication for name: {0}")]
    Authentication(String),
    #[error(transparent)]
    Internal(#[from] axum_login::Error<AuthBackend>),
}

#[cfg(test)]
mod test {
    use crate::auth::types::Permission::AssignAdm;
    use crate::auth::types::{Permission, Player, Role};
    use crate::types::UuidKey;
    use password_auth::generate_hash;
    use redb::{Key, Value};
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn test_player_encode_decode() {
        let password_hash = generate_hash("Test123$");
        let mut permissions = HashSet::new();
        permissions.insert(AssignAdm);
        let mut roles = HashSet::new();
        roles.insert(Uuid::new_v4());
        let orig_player = Player {
            uuid: Uuid::new_v4(),
            name: "test".to_string(),
            password_hash,
            permissions,
            roles,
            ..Default::default()
        };
        let encoded_player = Player::as_bytes(&orig_player);
        let decoded_player = Player::from_bytes(&encoded_player);
        assert_eq!(orig_player, decoded_player);
    }

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
