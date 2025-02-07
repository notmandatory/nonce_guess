use crate::auth::types::{Permission, Player, Role};
use crate::types::{InternalError, UuidKey};
use password_auth::generate_hash;
use redb::{
    Database, ReadTransaction, ReadableTable, ReadableTableMetadata, TableDefinition, TypeName,
    Value, WriteTransaction,
};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const UUID_PLAYER: TableDefinition<UuidKey, Player> = TableDefinition::new("auth_uuid_player");
const NAME_UUID: TableDefinition<String, UuidKey> = TableDefinition::new("auth_player_name_uuid");
const UUID_ROLE: TableDefinition<UuidKey, Role> = TableDefinition::new("auth_uuid_role");

#[derive(Debug, Clone)]
pub struct AuthDb(Arc<Database>);

impl AuthDb {
    pub fn new(db: Arc<Database>) -> Result<Self, InternalError> {
        let mut write_txn = db.begin_write()?;
        Self::init(&mut write_txn)?;
        write_txn.commit()?;
        Ok(AuthDb(db))
    }

    fn init(write_txn: &mut WriteTransaction) -> Result<(), InternalError> {
        let tables_empty = {
            let uuid_role = write_txn.open_table(UUID_ROLE)?;
            let uuid_player = write_txn.open_table(UUID_PLAYER)?;
            let name_uuid = write_txn.open_table(NAME_UUID)?;
            info!(
                "opened tables: {}, {}, {}",
                UUID_ROLE, UUID_PLAYER, NAME_UUID
            );
            uuid_role.is_empty()? && uuid_player.is_empty()? && name_uuid.is_empty()?
        };
        // if all tables are empty, insert admin user and admin role
        if tables_empty {
            let role_uuid = Uuid::new_v4();
            let admin_role = Role {
                uuid: role_uuid,
                name: "admin".to_string(),
                permissions: [Permission::AssignAdm, Permission::ChangeTarget].into(),
            };
            let password_hash = generate_hash("aBcD123$");
            let mut roles = HashSet::new();
            roles.insert(role_uuid);
            let admin = Player {
                uuid: Uuid::new_v4(),
                name: "admin".to_string(),
                password_hash,
                permissions: Default::default(),
                roles,
                ..Default::default()
            };
            AuthDb::insert_role(write_txn, admin_role)?;
            AuthDb::insert_player(write_txn, admin)?;
            info!("inserted admin_role and admin user");
        }
        Ok(())
    }

    pub fn begin_write(&self) -> Result<WriteTransaction, InternalError> {
        Ok(self.0.begin_write()?)
    }

    pub fn begin_read(&self) -> Result<ReadTransaction, InternalError> {
        Ok(self.0.begin_read()?)
    }

    pub fn insert_player(
        write_txn: &mut WriteTransaction,
        player: Player,
    ) -> Result<Option<Player>, InternalError> {
        let name_uuid = &mut write_txn.open_table(NAME_UUID)?;
        name_uuid
            .insert(&player.name, &UuidKey(player.uuid))
            .map_err(Into::<InternalError>::into)?;

        let uuid_player = &mut write_txn.open_table(UUID_PLAYER)?;
        uuid_player
            .insert(&UuidKey(player.uuid), player.clone())
            .map(|opt| opt.map(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub fn get_player_by_uuid(
        read_txn: &ReadTransaction,
        uuid_key: UuidKey,
    ) -> Result<Option<Player>, InternalError> {
        let uuid_player = read_txn.open_table(UUID_PLAYER)?;
        let player = uuid_player.get(&uuid_key)?.map(|ag| ag.value());
        Ok(player)
    }

    pub fn get_player_by_name(
        read_txn: &ReadTransaction,
        name: &str,
    ) -> Result<Option<Player>, InternalError> {
        let name_player_uuid = read_txn.open_table(NAME_UUID)?;
        let player = name_player_uuid
            .get(name.to_string())?
            .map(|ag| ag.value().0)
            .map(|uuid| AuthDb::get_player_by_uuid(read_txn, UuidKey(uuid)))
            .transpose()?
            .flatten();
        Ok(player)
    }

    pub fn get_players(read_txn: &ReadTransaction) -> Result<Vec<Player>, InternalError> {
        let uuid_player = read_txn.open_table(UUID_PLAYER)?;
        uuid_player
            .iter()?
            .map(|result| {
                result
                    .map(|(_, player_ag)| player_ag.value())
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<Player>, InternalError>>()
    }

    pub fn insert_role(
        write_txn: &mut WriteTransaction,
        role: Role,
    ) -> Result<Option<Role>, InternalError> {
        let uuid_key = UuidKey(role.uuid);
        let mut uuid_role = write_txn.open_table(UUID_ROLE)?;
        uuid_role
            .insert(&uuid_key, &role)
            .map(|opt| opt.map(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub fn get_role_by_uuid(
        read_txn: &ReadTransaction,
        uuid_key: UuidKey,
    ) -> Result<Option<Role>, InternalError> {
        let name_role = read_txn.open_table(UUID_ROLE)?;
        name_role
            .get(uuid_key)
            .map(|opt| opt.map(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub fn get_roles(read_txn: &ReadTransaction) -> Result<Vec<Role>, InternalError> {
        let uuid_role = read_txn.open_table(UUID_ROLE)?;
        uuid_role
            .iter()?
            .map(|result| {
                result
                    .map(|(_, role_ag)| role_ag.value())
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<Role>, InternalError>>()
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
