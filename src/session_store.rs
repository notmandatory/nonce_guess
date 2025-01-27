use async_trait::async_trait;
use log::warn;
use redb::{Database, Key, ReadableTable, TableDefinition, TypeName, Value};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tower_sessions::session_store::Error;
use tower_sessions::{
    cookie::time::OffsetDateTime,
    session::{Id, Record},
    session_store, ExpiredDeletion, SessionStore,
};

// TODO: extract this into it's own lib

const ID_RECORD: TableDefinition<IdKey, RecordValue> = TableDefinition::new("session_id_record");

/// Session store backed by redb
#[derive(Debug, Clone)]
pub struct RedbSessionStore {
    // the redb database which should be used for storage
    db: Arc<Database>,
}

impl RedbSessionStore {
    /// Create a new RedbStore using a [`Database`]
    pub fn new(db: Arc<Database>) -> Self {
        // TODO: call session store database migrations here
        Self { db }
    }

    pub async fn continuously_delete_expired(
        self,
        period: tokio::time::Duration,
    ) -> session_store::Result<()> {
        let mut interval = tokio::time::interval(period);
        interval.tick().await; // The first tick completes immediately; skip.
        loop {
            interval.tick().await;
            self.delete_expired().await?;
        }
    }

    pub fn save_blocking(db: Arc<Database>, record: Record) -> session_store::Result<()> {
        let write_txn = db
            .begin_write()
            .map_err(|e| Error::Backend(e.to_string()))?;
        {
            let mut id_record = write_txn
                .open_table(ID_RECORD)
                .map_err(|e| Error::Backend(e.to_string()))?;
            id_record
                .insert(&IdKey(record.id), &RecordValue(record))
                .map_err(|e| Error::Backend(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| Error::Backend(e.to_string()))?;
        Ok::<(), Error>(())
    }

    pub fn load_blocking(
        db: Arc<Database>,
        id_key: IdKey,
    ) -> session_store::Result<Option<Record>> {
        let read_txn = db.begin_read().map_err(|e| Error::Backend(e.to_string()))?;
        let id_record = read_txn
            .open_table(ID_RECORD)
            .map_err(|e| Error::Backend(e.to_string()))?;
        let opt_record = id_record
            .get(&id_key)
            .map_err(|e| Error::Backend(e.to_string()))?
            .map(|ag| ag.value().0);
        Ok(opt_record)
    }
}

#[async_trait]
impl SessionStore for RedbSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let db = self.db.clone();
        let mut record = record.clone();
        spawn_blocking(move || {
            let mut id_key = IdKey(record.id.clone());
            while Self::load_blocking(db.clone(), id_key)
                .unwrap_or(None)
                .is_some()
            {
                // Session ID collision mitigation.
                warn!("session record id collision: {}", &record.id.0);
                record.id = Id::default();
                id_key = IdKey(record.id.clone());
            }
            Self::save_blocking(db.clone(), record)
        })
        .await
        .map_err(|e| Error::Backend(e.to_string()))?
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let db = self.db.clone();
        let record = record.clone();
        spawn_blocking(move || RedbSessionStore::save_blocking(db, record))
            .await
            .map_err(|e| Error::Backend(e.to_string()))?
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        let db = self.db.clone();
        let id_key = IdKey(*id);
        spawn_blocking(move || match RedbSessionStore::load_blocking(db, id_key) {
            Ok(r) => Ok(r),
            Err(Error::Backend(_)) => Ok(None),
            Err(e) => Err(e),
        })
        .await
        .map_err(|e| Error::Backend(e.to_string()))?
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        let db = self.db.clone();
        let id_key = IdKey(*id);
        spawn_blocking(move || {
            let write_txn = db
                .begin_write()
                .map_err(|e| Error::Backend(e.to_string()))?;
            {
                let mut id_record = write_txn
                    .open_table(ID_RECORD)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                id_record
                    .remove(&id_key)
                    .map_err(|e| Error::Backend(e.to_string()))?;
            }
            write_txn
                .commit()
                .map_err(|e| Error::Backend(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::Backend(e.to_string()))?
    }
}

#[async_trait]
impl ExpiredDeletion for RedbSessionStore {
    /// Deletes expired sessions from the session store
    async fn delete_expired(&self) -> session_store::Result<()> {
        let db = self.db.clone();

        spawn_blocking(move || -> session_store::Result<()> {
            let now = OffsetDateTime::now_utc();
            let write_txn = db
                .begin_write()
                .map_err(|e| Error::Backend(e.to_string()))?;
            let mut id_record = write_txn
                .open_table(ID_RECORD)
                .map_err(|e| Error::Backend(e.to_string()))?;
            id_record
                .retain(|_, record| record.0.expiry_date >= now)
                .map_err(|e| Error::Backend(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| session_store::Error::Backend(e.to_string()))?
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RecordValue(pub Record);

impl Value for RecordValue {
    type SelfType<'a> = RecordValue;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(serialized_record: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let record: Record = ciborium::from_reader(serialized_record).unwrap();
        RecordValue(record)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let mut serialized_record = Vec::<u8>::new();
        ciborium::into_writer(&value.0, &mut serialized_record)
            .expect("Failed to serialize record");
        serialized_record
    }

    fn type_name() -> TypeName {
        TypeName::new("redb_session_store::RecordValue")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct IdKey(pub Id);

impl Value for IdKey {
    type SelfType<'a> = IdKey;
    type AsBytes<'a> = [u8; 16];

    fn fixed_width() -> Option<usize> {
        Some(16)
    }

    fn from_bytes<'a>(serialized_id: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let id = Id(i128::from_le_bytes(
            serialized_id[0..16].try_into().unwrap(),
        ));
        IdKey(id)
    }

    fn as_bytes<'a, 'b: 'a>(id_key: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let id = id_key.0;
        id.0.to_le_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new("redb_session_store::IdKey")
    }
}

impl Key for IdKey {
    fn compare(id1: &[u8], id2: &[u8]) -> Ordering {
        let id1 = i128::from_le_bytes(id1[0..16].try_into().unwrap());
        let id2 = i128::from_le_bytes(id2[0..16].try_into().unwrap());
        id1.cmp(&id2)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use std::ops::Add;
    use tempfile::NamedTempFile;
    use time::Duration;
    use tower_sessions::session::Record;

    #[test]
    fn test_record_value_encode_decode() {
        let mut data = HashMap::new();
        data.insert(
            "TestKey".to_string(),
            serde_json::Value::String("TestValue".to_string()),
        );
        let orig_record_value = RecordValue(Record {
            id: Default::default(),
            data,
            expiry_date: OffsetDateTime::now_utc().add(Duration::minutes(60)),
        });
        let encoded_record_value = RecordValue::as_bytes(&orig_record_value);
        let decoded_record_value = RecordValue::from_bytes(&encoded_record_value);
        assert_eq!(orig_record_value, decoded_record_value);
    }

    #[test]
    fn test_id_key_encode_decode() {
        let id = Id::default();
        let orig_id_key = IdKey(id);
        let encoded_id_key = IdKey::as_bytes(&orig_id_key);
        let decoded_id_key = IdKey::from_bytes(&encoded_id_key);
        assert_eq!(orig_id_key, decoded_id_key);
    }

    #[test]
    fn test_id_key_compare() {
        let id_key1 = IdKey(Id::default());
        let encoded_id_key1 = IdKey::as_bytes(&id_key1);
        let id_key2 = IdKey(Id::default());
        let encoded_id_key2 = IdKey::as_bytes(&id_key2);
        assert_eq!(
            IdKey::compare(&encoded_id_key1, &encoded_id_key2),
            id_key1.0 .0.cmp(&id_key2.0 .0)
        );
    }

    fn temp_db() -> Arc<Database> {
        let file = NamedTempFile::new().unwrap().into_temp_path();
        Arc::new(Database::create(file).unwrap())
    }

    #[tokio::test]
    async fn test_create_load_save_record() {
        let db = temp_db();
        let session_store = RedbSessionStore::new(db);

        // make sure no errors when loading from a new db
        assert_eq!(None, session_store.load(&Id::default()).await.unwrap());

        let mut data1 = HashMap::new();
        data1.insert(
            "TestKey1".to_string(),
            serde_json::Value::String("TestValue1".to_string()),
        );
        let mut record1 = Record {
            id: Default::default(),
            data: data1,
            expiry_date: OffsetDateTime::now_utc().add(Duration::minutes(60)),
        };
        session_store
            .create(&mut record1)
            .await
            .expect("created record1");

        let mut data2 = HashMap::new();
        data2.insert(
            "TestKey2".to_string(),
            serde_json::Value::String("TestValue2".to_string()),
        );
        let mut record2 = Record {
            id: Default::default(),
            data: data2,
            expiry_date: OffsetDateTime::now_utc().add(Duration::minutes(30)),
        };
        session_store
            .create(&mut record2)
            .await
            .expect("created record2");

        let mut loaded_record1 = session_store
            .load(&record1.id)
            .await
            .expect("loaded record1");
        assert_eq!(Some(record1), loaded_record1);

        let loaded_record2 = session_store
            .load(&record2.id)
            .await
            .expect("loaded record2");
        assert_eq!(Some(record2), loaded_record2);
        assert_eq!(None, session_store.load(&Id::default()).await.unwrap());

        let mut saved_record3 = loaded_record1.unwrap();
        saved_record3.data.insert(
            "TestKey3".to_string(),
            serde_json::Value::String("TestValue3".to_string()),
        );
        session_store
            .save(&saved_record3)
            .await
            .expect("saved record3");
        let loaded_record3 = session_store
            .load(&saved_record3.id)
            .await
            .expect("loaded record3");
        assert_eq!(Some(saved_record3), loaded_record3);
    }

    #[tokio::test]
    async fn test_create_load_delete_record() {
        let db = temp_db();
        let session_store = RedbSessionStore::new(db);

        let mut data1 = HashMap::new();
        data1.insert(
            "TestKey1".to_string(),
            serde_json::Value::String("TestValue1".to_string()),
        );
        let mut record1 = Record {
            id: Default::default(),
            data: data1,
            expiry_date: OffsetDateTime::now_utc().add(Duration::minutes(60)),
        };
        session_store
            .create(&mut record1)
            .await
            .expect("created record1");

        let loaded_record1 = session_store
            .load(&record1.id)
            .await
            .expect("loaded record1");
        assert_eq!(Some(record1.clone()), loaded_record1);

        session_store.delete(&record1.id).await.unwrap();
        let reloaded_record1 = session_store
            .load(&record1.id)
            .await
            .expect("reloaded record1");
        assert_eq!(None, reloaded_record1);
    }
}
