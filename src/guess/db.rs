use super::types::{Guess, GuessError};
use crate::types::InternalError;
use redb::{
    Database, Key, MultimapTableDefinition, ReadTransaction, ReadableTable, TableDefinition,
    TypeName, Value, WriteTransaction,
};
use std::cmp::Ordering;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const HEIGHT_NONCE: TableDefinition<u32, Option<u32>> = TableDefinition::new("guess_height_nonce");
const HEIGHT_GUESSES: MultimapTableDefinition<u32, Guess> =
    MultimapTableDefinition::new("guess_height_guesses");

#[derive(Debug, Clone)]
pub struct GuessDb(Arc<Database>);

impl GuessDb {
    pub fn new(db: Arc<Database>) -> Result<Self, InternalError> {
        let mut write_txn = db.begin_write()?;
        Self::init(&mut write_txn)?;
        write_txn.commit()?;
        Ok(GuessDb(db))
    }

    fn init(write_txn: &mut WriteTransaction) -> Result<(), InternalError> {
        // open tables to make sure they exist
        write_txn.open_table(HEIGHT_NONCE)?;
        write_txn.open_multimap_table(HEIGHT_GUESSES)?;
        info!("opened tables: {}, {}", HEIGHT_NONCE, HEIGHT_GUESSES);
        Ok(())
    }

    pub fn begin_write(&self) -> Result<WriteTransaction, InternalError> {
        Ok(self.0.begin_write()?)
    }

    pub fn begin_read(&self) -> Result<ReadTransaction, InternalError> {
        Ok(self.0.begin_read()?)
    }

    pub fn insert_target(
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

    pub fn get_target_nonce(
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

    pub fn get_last_target_nonce(
        read_txn: &ReadTransaction,
    ) -> Result<Option<(u32, Option<u32>)>, InternalError> {
        let height_nonce = read_txn.open_table(HEIGHT_NONCE)?;
        height_nonce
            .last()
            .map(|opt| opt.map(|(k_ag, v_ag)| (k_ag.value(), v_ag.value())))
            .map_err(Into::into)
    }

    pub fn replace_target(
        write_txn: &mut WriteTransaction,
        old_height: u32,
        new_height: u32,
    ) -> Result<(), InternalError> {
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
        Ok(())
    }

    pub fn remove_target_nonce(
        write_txn: &mut WriteTransaction,
        height: u32,
    ) -> Result<Option<u32>, InternalError> {
        let mut height_nonce = write_txn.open_table(HEIGHT_NONCE)?;
        height_nonce
            .remove(height)
            .map(|opt| opt.and_then(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub fn insert_guess(
        write_txn: &mut WriteTransaction,
        height: u32,
        guess: Guess,
    ) -> Result<bool, InternalError> {
        let mut height_guesses = write_txn.open_multimap_table(HEIGHT_GUESSES)?;
        height_guesses.insert(height, &guess).map_err(Into::into)
    }

    // check if a player has made any guess for the given target height
    pub fn any_guess(
        read_txn: &ReadTransaction,
        height: u32,
        player_uuid: Uuid,
    ) -> Result<bool, GuessError> {
        let target_opt =
            Self::get_target_nonce(read_txn, height).map_err(Into::<GuessError>::into)?;
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

    pub fn target_guesses(
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
