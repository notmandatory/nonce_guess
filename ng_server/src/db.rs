use crate::error::Error;
use axum::async_trait;
use ng_model::{Guess, Target};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Sqlite, Transaction};

#[async_trait]
pub trait Db {
    /// Insert new target block
    async fn insert_target(&mut self, block: u32) -> Result<(), Error>;

    /// Select current target block
    async fn select_current_target(&mut self) -> Result<Target, Error>;

    /// Set current target block nonce
    async fn set_current_nonce(&mut self, nonce: u32) -> Result<(), Error>;

    /// Insert new nonce guess
    async fn insert_guess(&mut self, guess: Guess) -> Result<(), Error>;

    /// Select guesses for target block
    async fn select_guesses(&mut self, block: u32) -> Result<Vec<Guess>, Error>;
}

#[async_trait]
impl<'c> Db for Transaction<'c, Sqlite> {
    async fn insert_target(&mut self, block: u32) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO target (block, nonce) VALUES (?, NULL)").bind(block);
        query
            .execute(&mut *self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn select_current_target(&mut self) -> Result<Target, Error> {
        let query = sqlx::query_as::<Sqlite, TargetRow>(
            "SELECT * FROM target WHERE block IS (SELECT MAX(block) FROM target)",
        );
        query
            .fetch_one(self)
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
            .execute(&mut *self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn insert_guess(&mut self, guess: Guess) -> Result<(), Error> {
        let query = sqlx::query("INSERT INTO guess (player_name, block, nonce) VALUES (?, ?, ?)")
            .bind(guess.name)
            .bind(guess.block)
            .bind(guess.nonce);
        query
            .execute(&mut *self)
            .await
            .map(|_| ())
            .map_err(|err| err.into())
    }

    async fn select_guesses(&mut self, block: u32) -> Result<Vec<Guess>, Error> {
        let query = sqlx::query_as::<Sqlite, GuessRow>(
            "SELECT * FROM guess WHERE block IS ? ORDER BY nonce",
        )
        .bind(block);
        query
            .fetch_all(self)
            .await
            .map_err(|err| err.into())
            .map(|gv| gv.into_iter().map(|gr| gr.0).collect())
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
        let block = row.get::<u32, usize>(1);
        let nonce = row.get::<u32, usize>(2);
        Ok(GuessRow(Guess {
            name: player_name,
            block,
            nonce,
        }))
    }
}
