use crate::guess::types::{Block, Guess, GuessError};
use crate::types::InternalError;
use redb::{
    Database, MultimapTableDefinition, ReadTransaction, ReadableTable, TableDefinition,
    WriteTransaction,
};
use reqwest::Url;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::info;
use uuid::Uuid;

const HEIGHT_NONCE: TableDefinition<u32, Option<u32>> = TableDefinition::new("guess_height_nonce");
const HEIGHT_GUESSES: MultimapTableDefinition<u32, Guess> =
    MultimapTableDefinition::new("guess_height_guesses");

#[derive(Debug, Clone)]
pub struct GuessBackend {
    pub db: Arc<Database>,
    pub http_client: reqwest::Client,
    pub mempool_url: Url,
}

impl GuessBackend {
    pub fn new(
        db: Arc<Database>,
        http_client: reqwest::Client,
        mempool_url: Url,
    ) -> Result<Self, InternalError> {
        Self::init_db(&db)?;
        Ok(Self {
            db,
            http_client,
            mempool_url,
        })
    }

    pub fn init_db(db: &Arc<Database>) -> Result<(), InternalError> {
        let db = db.clone();
        let write_txn = db.begin_write()?;
        // open tables to make sure they exist
        write_txn.open_table(HEIGHT_NONCE)?;
        write_txn.open_multimap_table(HEIGHT_GUESSES)?;
        info!("opened tables: {}, {}", HEIGHT_NONCE, HEIGHT_GUESSES);
        write_txn.commit()?;
        Ok(())
    }

    pub async fn insert_target(
        &self,
        height: u32,
        nonce: Option<u32>,
    ) -> Result<Option<u32>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut write_txn = db.begin_write()?;
            let insert_target_result = Self::insert_target_blocking(&mut write_txn, height, nonce);
            write_txn.commit()?;
            insert_target_result
        })
        .await?
    }

    fn insert_target_blocking(
        write_txn: &mut WriteTransaction,
        height: u32,
        nonce: Option<u32>,
    ) -> Result<Option<u32>, InternalError> {
        let mut height_nonce = write_txn.open_table(HEIGHT_NONCE)?;
        height_nonce
            .insert(height, nonce)
            .map(|opt| opt.and_then(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub async fn get_target_nonce(
        &self,
        height: u32,
    ) -> Result<Option<Option<u32>>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read().map_err(Into::<InternalError>::into)?;
            Self::get_target_nonce_blocking(&read_txn, height)
        })
        .await
        .map_err(Into::<InternalError>::into)?
    }

    fn get_target_nonce_blocking(
        read_txn: &ReadTransaction,
        height: u32,
    ) -> Result<Option<Option<u32>>, InternalError> {
        let height_nonce = read_txn
            .open_table(HEIGHT_NONCE)
            .map_err(Into::<InternalError>::into)?;
        height_nonce
            .get(height)
            .map(|opt| opt.map(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub async fn get_last_target_nonce(&self) -> Result<Option<(u32, Option<u32>)>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read()?;
            let height_nonce = read_txn.open_table(HEIGHT_NONCE)?;
            height_nonce
                .last()
                .map(|opt| opt.map(|(k_ag, v_ag)| (k_ag.value(), v_ag.value())))
                .map_err(Into::into)
        })
        .await?
    }

    pub async fn remove_target_nonce(&self, height: u32) -> Result<Option<u32>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut write_txn = db.begin_write()?;
            let remove_target_result = Self::remove_target_nonce_blocking(&mut write_txn, height);
            write_txn.commit()?;
            remove_target_result
        })
        .await?
    }

    fn remove_target_nonce_blocking(
        write_txn: &mut WriteTransaction,
        height: u32,
    ) -> Result<Option<u32>, InternalError> {
        let mut height_nonce = write_txn.open_table(HEIGHT_NONCE)?;
        height_nonce
            .remove(height)
            .map(|opt| opt.and_then(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub async fn replace_target(
        &self,
        old_height: u32,
        new_height: u32,
    ) -> Result<(), InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let write_txn = db.begin_write().map_err(Into::<InternalError>::into)?;
            {
                let mut height_nonce = write_txn
                    .open_table(HEIGHT_NONCE)
                    .map_err(Into::<InternalError>::into)?;
                // remove old target nonce
                let _ = height_nonce
                    .remove(&old_height)
                    //.map(|nonce_res| nonce_res.map(|ag| ag.value()).flatten())
                    .map_err(Into::<InternalError>::into)?;
                // create new target nonce
                height_nonce
                    .insert(new_height, None)
                    .map_err(Into::<InternalError>::into)?;

                let mut height_guesses = write_txn
                    .open_multimap_table(HEIGHT_GUESSES)
                    .map_err(Into::<InternalError>::into)?;
                // remove guesses from old target
                let guesses = height_guesses
                    .remove_all(old_height)
                    .map(|guess| {
                        guess
                            .flat_map(|ag_res| {
                                ag_res
                                    .map(|ag| ag.value())
                                    .map_err(Into::<InternalError>::into)
                            })
                            .collect::<Vec<Guess>>()
                    })
                    .map_err(Into::<InternalError>::into)?;
                // insert guesses from old target into new target
                guesses.iter().try_for_each(|guess| {
                    let present_res = height_guesses
                        .insert(new_height, guess)
                        .map_err(Into::<InternalError>::into);
                    present_res.map(|_| ())
                })?;
            }
            write_txn.commit().map_err(Into::<InternalError>::into)
        })
        .await?
        .map_err(Into::<InternalError>::into)
    }

    pub async fn insert_guess(&self, height: u32, guess: Guess) -> Result<bool, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let write_txn = db.begin_write()?;
            let insert_target_result = {
                let mut height_guesses = write_txn.open_multimap_table(HEIGHT_GUESSES)?;
                height_guesses.insert(height, &guess).map_err(Into::into)
            };
            write_txn.commit()?;
            insert_target_result
        })
        .await?
    }

    pub async fn any_guess(&self, height: u32, player_uuid: Uuid) -> Result<bool, GuessError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read().map_err(Into::<InternalError>::into)?;
            Self::any_guess_blocking(&read_txn, height, player_uuid)
        })
        .await
        .map_err(Into::<InternalError>::into)?
    }

    // check if a player has made any guess for the given target height
    fn any_guess_blocking(
        read_txn: &ReadTransaction,
        height: u32,
        player_uuid: Uuid,
    ) -> Result<bool, GuessError> {
        let target_opt =
            Self::get_target_nonce_blocking(read_txn, height).map_err(Into::<GuessError>::into)?;
        if target_opt.is_none() {
            return Err(GuessError::MissingTarget(height));
        };
        let height_guesses = read_txn
            .open_multimap_table(HEIGHT_GUESSES)
            .map_err(Into::<InternalError>::into)?;
        let mut guesses = height_guesses
            .get(height)
            .map_err(Into::<InternalError>::into)?;

        guesses.try_fold(false, |any, guess_res| {
            guess_res
                .map(|ag| any || ag.value().player == player_uuid)
                .map_err(Into::<InternalError>::into)
                .map_err(Into::<GuessError>::into)
        })
    }

    pub async fn target_guesses(&self, height: u32) -> Result<Vec<Guess>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read().map_err(Into::<InternalError>::into)?;
            Self::target_guesses_blocking(&read_txn, height)
        })
        .await
        .map_err(Into::<InternalError>::into)?
    }

    fn target_guesses_blocking(
        read_txn: &ReadTransaction,
        height: u32,
    ) -> Result<Vec<Guess>, InternalError> {
        let height_guesses = read_txn
            .open_multimap_table(HEIGHT_GUESSES)
            .map_err(Into::<InternalError>::into)?;
        height_guesses
            .get(height)
            .map(|guess| {
                guess
                    .flat_map(|ag_res| {
                        ag_res
                            .map(|ag| ag.value())
                            .map_err(Into::<InternalError>::into)
                    })
                    .collect::<Vec<Guess>>()
            })
            .map_err(Into::<InternalError>::into)
    }
}

pub async fn continuously_update_target_nonce(
    guess_backend: Arc<GuessBackend>,
) -> Result<(), InternalError> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    interval.tick().await; // The first tick completes immediately; skip.
    loop {
        interval.tick().await;
        update_target_nonce(guess_backend.clone())
            .await
            .map_err(Into::<InternalError>::into)?
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
