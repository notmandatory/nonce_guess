use std::str::FromStr;
use crate::error::Error;
use crate::model::{Guess, Target};
use axum::async_trait;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Sqlite, Transaction};
use webauthn_rs::prelude::{Passkey, Uuid};

#[async_trait]
pub trait Db {
    /// Insert new player
    async fn insert_player(&mut self, name: &String, uuid: &Uuid) -> Result<(), Error>;

    // Select player UUID
    async fn select_player_uuid(&mut self, name: &String) -> Result<Uuid, Error>;

    /// Insert new player passkey
    async fn insert_player_passkey(&mut self, uuid: &Uuid, passkey: &Passkey) -> Result<(), Error>;

    /// Select player passkeys
    async fn select_player_passkeys(&mut self, uuid: &Uuid) -> Result<Vec<Passkey>, Error>;

    /// Insert new target block
    async fn insert_target(&mut self, block: u32) -> Result<(), Error>;

    /// Select current target block
    async fn select_current_target(&mut self) -> Result<Target, Error>;

    /// Set current target block nonce
    async fn set_current_nonce(&mut self, nonce: u32) -> Result<(), Error>;

    /// Insert new nonce guess
    async fn insert_guess(&mut self, uuid: &Uuid, block: Option<u32>, nonce: u32) -> Result<(), Error>;

    /// Select guesses for target block
    async fn select_block_guesses(&mut self, block: u32) -> Result<Vec<Guess>, Error>;

    /// Select guesses with no found target block
    async fn select_guesses(&mut self) -> Result<Vec<Guess>, Error>;

    /// Set block for guesses without a set block
    async fn set_guesses_block(&mut self, block: u32) -> Result<(), Error>;
}

#[async_trait]
impl<'c> Db for Transaction<'c, Sqlite> {
    async fn insert_player(&mut self, name: &String, uuid: &Uuid) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO player (uuid, name) VALUES (?, ?)")
            .bind(uuid.to_string())
            .bind(name.clone());
        query
            .execute(&mut **self)
            .await
            .map(|_| ())
            .map_err(|err| <sqlx::Error as Into<Error>>::into(err))
    }

    async fn select_player_uuid(&mut self, name: &String) -> Result<Uuid, Error> {
        let query = sqlx::query::<Sqlite>("SELECT uuid FROM player WHERE name IS ?")
            .bind(name.clone());
        query
            .fetch_one(&mut **self)
            .await
            .map_err(|err| {dbg!(&err); err.into()})
            .map(|row| row.get::<String, usize>(0))
            .map(|uuid| Uuid::from_str(uuid.as_str()).unwrap())
    }

    async fn insert_player_passkey(&mut self, uuid: &Uuid, passkey: &Passkey) -> Result<(), Error> {

        let mut passkey_cbor = Vec::new();
        ciborium::into_writer(&passkey, &mut passkey_cbor).unwrap();

        let query = sqlx::query("INSERT INTO auth (uuid, passkey) VALUES (?, ?)")
            .bind(uuid.to_string())
            .bind(passkey_cbor.as_slice());
        query
            .execute(&mut **self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn select_player_passkeys(&mut self, uuid: &Uuid) -> Result<Vec<Passkey>, Error> {
        let query = sqlx::query::<Sqlite>(
            "SELECT passkey FROM auth WHERE uuid IS ?",
        )
        .bind(uuid.to_string());
        query
            .fetch_all(&mut **self)
            .await
            .map_err(|err| err.into())
            .map(|rows| {
                rows.into_iter()
                    .map(|row| row.get::<Vec<u8>, usize>(0))
                    .map(|passkey| {
                        ciborium::from_reader(&passkey[..]).unwrap()
                    })
                    .collect()
            })
    }

    async fn insert_target(&mut self, block: u32) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO target (block, nonce) VALUES (?, NULL)").bind(block);
        query
            .execute(&mut **self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn select_current_target(&mut self) -> Result<Target, Error> {
        let query = sqlx::query_as::<Sqlite, TargetRow>(
            "SELECT * FROM target WHERE block IS (SELECT MAX(block) FROM target)",
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
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn insert_guess(&mut self, uuid: &Uuid, block: Option<u32>, nonce: u32) -> Result<(), Error> {

        let query = sqlx::query("INSERT INTO guess (uuid, block, nonce) VALUES (?, ?, ?)")
            .bind(uuid.to_string())
            .bind(block)
            .bind(nonce);
        query
            .execute(&mut **self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn select_block_guesses(&mut self, block: u32) -> Result<Vec<Guess>, Error> {
        let query = sqlx::query_as::<Sqlite, GuessRow>(
            "SELECT name, block, nonce FROM guess \
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
            "SELECT name, block, nonce FROM guess \
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
        let query = sqlx::query("UPDATE guess set block = ? WHERE block IS null").bind(block);
        query
            .execute(&mut **self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
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
        let player_name = row.get::<String, usize>(0);
        let block = row.get::<Option<u32>, usize>(1);
        let nonce = row.get::<u32, usize>(2);
        Ok(GuessRow(Guess {
            name: player_name,
            block,
            nonce,
        }))
    }
}