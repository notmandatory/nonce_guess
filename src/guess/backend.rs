use super::db::GuessDb;
use super::types::{Block, Guess, GuessError};
use crate::types::InternalError;
use redb::Database;
use reqwest::Url;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GuessBackend {
    pub guess_db: GuessDb,
    pub http_client: reqwest::Client,
    pub mempool_url: Url,
}

impl GuessBackend {
    pub fn new(
        db: Arc<Database>,
        http_client: reqwest::Client,
        mempool_url: Url,
    ) -> Result<Self, InternalError> {
        let guess_db = GuessDb::new(db)?;
        Ok(Self {
            guess_db,
            http_client,
            mempool_url,
        })
    }

    pub async fn insert_target(
        &self,
        height: u32,
        nonce: Option<u32>,
    ) -> Result<Option<u32>, InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let mut write_txn = guess_db.begin_write()?;
            let insert_target_result = GuessDb::insert_target(&mut write_txn, height, nonce);
            write_txn.commit()?;
            insert_target_result
        })
        .await?
    }

    pub async fn get_target_nonce(
        &self,
        height: u32,
    ) -> Result<Option<Option<u32>>, InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let read_txn = guess_db.begin_read()?;
            GuessDb::get_target_nonce(&read_txn, height)
        })
        .await
        .map_err(Into::<InternalError>::into)?
    }

    pub async fn get_last_target_nonce(&self) -> Result<Option<(u32, Option<u32>)>, InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let read_txn = guess_db.begin_read()?;
            GuessDb::get_last_target_nonce(&read_txn)
        })
        .await?
    }

    pub async fn remove_target_nonce(&self, height: u32) -> Result<Option<u32>, InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let mut write_txn = guess_db.begin_write()?;
            let remove_target_result = GuessDb::remove_target_nonce(&mut write_txn, height);
            write_txn.commit()?;
            remove_target_result
        })
        .await?
    }

    pub async fn replace_target(
        &self,
        old_height: u32,
        new_height: u32,
    ) -> Result<(), InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let mut write_txn = guess_db.begin_write()?;
            GuessDb::replace_target(&mut write_txn, old_height, new_height)?;
            write_txn.commit().map_err(Into::<InternalError>::into)
        })
        .await?
    }

    pub async fn insert_guess(&self, height: u32, guess: Guess) -> Result<bool, InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let mut write_txn = guess_db.begin_write()?;
            let insert_guess_result = GuessDb::insert_guess(&mut write_txn, height, guess.clone());
            write_txn.commit()?;
            insert_guess_result
        })
        .await?
    }

    pub async fn any_guess(&self, height: u32, player_uuid: Uuid) -> Result<bool, GuessError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let read_txn = guess_db.begin_read()?;
            GuessDb::any_guess(&read_txn, height, player_uuid)
        })
        .await
        .map_err(Into::<InternalError>::into)?
    }

    pub async fn target_guesses(&self, height: u32) -> Result<Vec<Guess>, InternalError> {
        let guess_db = self.guess_db.clone();
        spawn_blocking(move || {
            let read_txn = guess_db.begin_read()?;
            GuessDb::target_guesses(&read_txn, height)
        })
        .await
        .map_err(Into::<InternalError>::into)?
    }
}

pub async fn continuously_update_target_nonce(
    guess_backend: Arc<GuessBackend>,
) -> Result<(), InternalError> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    interval.tick().await; // The first tick completes immediately; skip.
    loop {
        interval.tick().await;
        update_target_nonce(guess_backend.clone()).await?
    }
}

async fn update_target_nonce(guess_backend: Arc<GuessBackend>) -> Result<(), InternalError> {
    if let Some((height, None)) = guess_backend.get_last_target_nonce().await? {
        let client = guess_backend.http_client.clone();
        let block_height_response = client
            .get(format!(
                "{}/api/block-height/{}",
                guess_backend.mempool_url, height
            ))
            .send()
            .await?;
        if block_height_response.status().is_success() {
            let block_hash = block_height_response.text().await?;
            let block_response = client
                .get(format!(
                    "{}/api/block/{}",
                    guess_backend.mempool_url, block_hash
                ))
                .send()
                .await?;
            if block_response.status().is_success() {
                let block: Block = block_response.json().await?;
                let nonce = block.nonce;
                guess_backend.insert_target(height, Some(nonce)).await?;
                info!("updated target nonce for height {} to {}", height, nonce);
            }
        }
        info!("checked target nonce for height {}", height);
        Ok::<(), InternalError>(())
    } else {
        Ok::<(), InternalError>(())
    }
}
