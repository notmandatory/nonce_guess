use crate::model::{Guess, Player, Target};
use crate::web::auth::{Permission, Role};
use axum::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Sqlite, Transaction};
use std::collections::HashSet;
use std::str::FromStr;
use uuid::Uuid;
use webauthn_rs::prelude::Passkey;

#[async_trait]
pub trait Db {
    /// Select if any player exist in the system.
    async fn select_exists_player(&mut self) -> Result<bool, Error>;

    /// Insert new player.
    async fn insert_player(&mut self, name: &String, uuid: &Uuid) -> Result<(), Error>;

    /// Select player by UUID.
    async fn select_player_by_uuid(&mut self, uuid: &Uuid) -> Result<Option<Player>, Error>;

    // /// Select player by credential id.
    // async fn select_player_by_credential(
    //     &mut self,
    //     cred_id: &CredentialID,
    // ) -> Result<Option<Player>, Error>;

    /// Select player UUID.
    async fn select_player_uuid(&mut self, name: &String) -> Result<Uuid, Error>;

    /// Select player name.
    async fn select_player_name(&mut self, uuid: &Uuid) -> Result<String, Error>;

    /// Insert new player passkey.
    async fn insert_player_passkey(&mut self, uuid: &Uuid, passkey: &Passkey) -> Result<(), Error>;

    /// Select player passkeys.
    async fn select_player_passkeys(&mut self, uuid: &Uuid) -> Result<Vec<Passkey>, Error>;

    // /// Insert player permission.
    // async fn insert_permission(
    //     &mut self,
    //     uuid: &Uuid,
    //     permissions: &Permission,
    // ) -> Result<(), Error>;

    /// Select player permissions.
    async fn select_permissions(&mut self, cred_id: &Uuid) -> Result<HashSet<Permission>, Error>;

    /// Insert player role.
    async fn insert_role(&mut self, uuid: &Uuid, role: &Role) -> Result<(), Error>;

    /// Select player roles.
    async fn select_roles(&mut self, cred_id: &Uuid) -> Result<HashSet<Role>, Error>;

    /// Insert new target block.
    async fn insert_target(&mut self, block: u32) -> Result<(), Error>;

    /// Select current target block.
    async fn select_current_target(&mut self) -> Result<Target, Error>;

    /// Set current target block nonce.
    async fn set_current_nonce(&mut self, nonce: u32) -> Result<(), Error>;

    /// Insert new nonce guess.
    async fn insert_guess(
        &mut self,
        uuid: &Uuid,
        block: Option<u32>,
        nonce: u32,
    ) -> Result<(), Error>;

    /// Select guesses for target block.
    async fn select_block_guesses(&mut self, block: u32) -> Result<Vec<Guess>, Error>;

    /// Select guesses with no found target block.
    async fn select_guesses(&mut self) -> Result<Vec<Guess>, Error>;

    /// Set block for guesses without a set block.
    async fn set_guesses_block(&mut self, block: u32) -> Result<(), Error>;
}

#[async_trait]
impl<'c> Db for Transaction<'c, Sqlite> {
    async fn select_exists_player(&mut self) -> Result<bool, Error> {
        let exists_query = sqlx::query::<Sqlite>("SELECT EXISTS (SELECT 1 FROM player LIMIT 1)");

        exists_query
            .fetch_one(&mut **self)
            .await
            .map_err(Error::Sqlx)
            .map(|row| row.get::<bool, usize>(0))
    }

    async fn insert_player(&mut self, name: &String, uuid: &Uuid) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO player (uuid, name) VALUES (?, ?)")
            .bind(uuid.to_string())
            .bind(name.clone());
        query
            .execute(&mut **self)
            .await
            .map_err(Error::from)
            .map(|_| ())
    }

    async fn select_player_by_uuid(&mut self, uuid: &Uuid) -> Result<Option<Player>, Error> {
        let name_query =
            sqlx::query::<Sqlite>("SELECT name FROM player WHERE uuid IS ?").bind(uuid.to_string());

        let name = name_query
            .fetch_optional(&mut **self)
            .await
            .map_err(Error::Sqlx)?
            .map(|row| row.get::<String, usize>(0));

        if let Some(name) = name {
            let uuid = uuid.clone();
            let passkeys = self.select_player_passkeys(&uuid).await?;
            Ok(Some(Player {
                uuid,
                name,
                passkeys,
            }))
        } else {
            Ok(None)
        }
    }

    // async fn select_player_by_credential(
    //     &mut self,
    //     cred_id: &CredentialID,
    // ) -> Result<Option<Player>, Error> {
    //     let name_uuid_query = sqlx::query::<Sqlite>("SELECT player.name, player.uuid FROM player JOIN authn ON player.uuid = authn.uuid WHERE authn.cred_id IS ?")
    //             .bind(cred_id.as_slice());

    //     let name_uuid = name_uuid_query
    //         .fetch_optional(&mut **self)
    //         .await
    //         .map_err(Error::from)
    //         .map(|opt_row| {
    //             opt_row.map(|row| {
    //                 let name = row.get::<String, usize>(0);
    //                 let uuid = row.get::<String, usize>(1);
    //                 (name, uuid)
    //             })
    //         })?;

    //     if let Some((name, uuid)) = name_uuid {
    //         let uuid = Uuid::from_str(uuid.as_str()).map_err(Error::from)?;
    //         let passkeys = self.select_player_passkeys(&uuid).await?;
    //         Ok(Some(Player {
    //             uuid,
    //             name,
    //             passkeys,
    //         }))
    //     } else {
    //         Ok(None)
    //     }
    // }

    async fn select_player_uuid(&mut self, name: &String) -> Result<Uuid, Error> {
        let query =
            sqlx::query::<Sqlite>("SELECT uuid FROM player WHERE name IS ?").bind(name.clone());
        query
            .fetch_one(&mut **self)
            .await
            .map_err(Error::from)
            .map(|row| row.get::<String, usize>(0))
            .and_then(|uuid| Uuid::from_str(uuid.as_str()).map_err(Error::from))
    }

    async fn select_player_name(&mut self, uuid: &Uuid) -> Result<String, Error> {
        dbg!(uuid);
        let query =
            sqlx::query::<Sqlite>("SELECT name FROM player WHERE uuid IS ?").bind(uuid.to_string());
        query
            .fetch_one(&mut **self)
            .await
            .map_err(Error::from)
            .map(|row| row.get::<String, usize>(0))
            .map_err(Error::from)
    }

    async fn insert_player_passkey(&mut self, uuid: &Uuid, passkey: &Passkey) -> Result<(), Error> {
        let cred_id = passkey.cred_id();
        let mut passkey_cbor = Vec::new();
        ciborium::into_writer(passkey, &mut passkey_cbor).map_err(Error::CborSer)?;

        let query = sqlx::query("INSERT INTO authn (cred_id, uuid, passkey) VALUES (?, ?, ?)")
            .bind(cred_id.as_slice())
            .bind(uuid.to_string())
            .bind(passkey_cbor.as_slice());
        query
            .execute(&mut **self)
            .await
            .map_err(Error::from)
            .map(|_| ())
    }

    async fn select_player_passkeys(&mut self, uuid: &Uuid) -> Result<Vec<Passkey>, Error> {
        let query = sqlx::query::<Sqlite>("SELECT passkey FROM authn WHERE uuid IS ?")
            .bind(uuid.to_string());
        query
            .fetch_all(&mut **self)
            .await
            .map_err(Error::from)
            .and_then(|rows| {
                rows.into_iter()
                    .map(|row| row.get::<Vec<u8>, usize>(0))
                    .map(|passkey| ciborium::from_reader(&passkey[..]).map_err(Error::CborDe))
                    .collect()
            })
    }

    // async fn insert_permission(
    //     &mut self,
    //     uuid: &Uuid,
    //     permission: &Permission,
    // ) -> Result<(), Error> {
    //     let query = sqlx::query::<Sqlite>("INSERT INTO authz (uuid, permission) VALUES (?, ?)")
    //         .bind(uuid.to_string())
    //         .bind(permission.to_string());

    //     query
    //         .execute(&mut **self)
    //         .await
    //         .map_err(Error::from)
    //         .map(|_| ())
    // }

    async fn select_permissions(&mut self, uuid: &Uuid) -> Result<HashSet<Permission>, Error> {
        let query =
            sqlx::query_as::<Sqlite, Permission>("SELECT permission FROM authz WHERE uuid IS ?")
                .bind(uuid.to_string());

        query
            .fetch_all(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|perms| perms.into_iter().collect())
    }

    async fn insert_role(&mut self, uuid: &Uuid, role: &Role) -> Result<(), Error> {
        let query = sqlx::query::<Sqlite>("INSERT INTO player_role (uuid, role) VALUES (?, ?)")
            .bind(uuid.to_string())
            .bind(role.to_string());

        query
            .execute(&mut **self)
            .await
            .map_err(Error::from)
            .map(|_| ())
    }

    async fn select_roles(&mut self, uuid: &Uuid) -> Result<HashSet<Role>, Error> {
        let query = sqlx::query_as::<Sqlite, Role>("SELECT role FROM player_role WHERE uuid IS ?")
            .bind(uuid.to_string());

        query
            .fetch_all(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|roles| roles.into_iter().collect())
    }

    async fn insert_target(&mut self, block: u32) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO target (block, nonce) VALUES (?, NULL)").bind(block);
        query
            .execute(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|_| ())
    }

    async fn select_current_target(&mut self) -> Result<Target, Error> {
        let query = sqlx::query_as::<Sqlite, TargetRow>(
            "SELECT * FROM target WHERE block IS (SELECT MAX(block) FROM target)", // AND nonce IS NULL",
        );
        query
            .fetch_one(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|tr| tr.0)
    }

    async fn set_current_nonce(&mut self, nonce: u32) -> Result<(), Error> {
        let query = sqlx::query(
            "UPDATE target SET nonce = ? WHERE block IS (SELECT MAX(block) FROM target)",
        )
        .bind(nonce);
        query
            .execute(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|_| ())
    }

    async fn insert_guess(
        &mut self,
        uuid: &Uuid,
        block: Option<u32>,
        nonce: u32,
    ) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO guess (uuid, block, nonce) VALUES (?, ?, ?)")
            .bind(uuid.to_string())
            .bind(block)
            .bind(nonce);
        query
            .execute(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|_| ())
    }

    async fn select_block_guesses(&mut self, block: u32) -> Result<Vec<Guess>, Error> {
        let query = sqlx::query_as::<Sqlite, GuessRow>(
            "SELECT guess.uuid, name, block, nonce FROM guess \
                 INNER JOIN player ON player.uuid = guess.uuid \
                 WHERE block IS ? \
                 ORDER BY nonce",
        )
        .bind(block);
        query
            .fetch_all(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|gv| gv.into_iter().map(|gr| gr.0).collect())
    }

    async fn select_guesses(&mut self) -> Result<Vec<Guess>, Error> {
        let query = sqlx::query_as::<Sqlite, GuessRow>(
            "SELECT player.uuid, name, block, nonce FROM guess \
            INNER JOIN player ON player.uuid = guess.uuid \
            WHERE block IS null \
            ORDER BY nonce",
        );
        query
            .fetch_all(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|gv| gv.into_iter().map(|gr| gr.0).collect())
    }

    async fn set_guesses_block(&mut self, block: u32) -> Result<(), Error> {
        let query = sqlx::query(
            "UPDATE guess set block = ? \
            WHERE block IS null \
            OR block IN (SELECT block FROM target WHERE nonce IS NULL)",
        )
        .bind(block);
        query
            .execute(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|_| ())
    }
}

struct TargetRow(Target);

impl FromRow<'_, SqliteRow> for TargetRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let block = row.get::<u32, usize>(0);
        let nonce = row.get::<Option<u32>, usize>(1);
        Ok(TargetRow(Target { block, nonce }))
    }
}

struct GuessRow(Guess);

impl FromRow<'_, SqliteRow> for GuessRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let player_uuid = row.get::<String, usize>(0);
        let player_name = row.get::<String, usize>(1);
        let block = row.get::<Option<u32>, usize>(2);
        let nonce = row.get::<u32, usize>(3);
        Ok(GuessRow(Guess {
            uuid: Uuid::parse_str(player_uuid.as_str()).expect("uuid"),
            name: player_name,
            block,
            nonce,
        }))
    }
}

impl FromRow<'_, SqliteRow> for Permission {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let permission_string = row.get::<&str, usize>(0);
        Permission::from_str(permission_string).map_err(|_| {
            sqlx::Error::Decode(Box::<super::error::Error>::new(
                super::error::Error::InvalidPermission(permission_string.to_string()),
            ))
        })
    }
}

impl FromRow<'_, SqliteRow> for Role {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let role_string = row.get::<&str, usize>(0);
        Role::from_str(role_string).map_err(|_| {
            sqlx::Error::Decode(Box::<super::error::Error>::new(
                super::error::Error::InvalidRole(role_string.to_string()),
            ))
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("uuid: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("cbor serialize: {0}")]
    CborSer(#[from] ciborium::ser::Error<std::io::Error>),
    #[error("cbor deserialize: {0}")]
    CborDe(#[from] ciborium::de::Error<std::io::Error>),
    #[error("serde json: {0}")]
    SerdeJson(#[from] serde_json::Error),
}
