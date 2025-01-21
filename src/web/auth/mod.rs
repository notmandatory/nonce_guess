pub mod types;
pub mod web;

use async_trait::async_trait;
use axum_login::{AuthnBackend, AuthzBackend, UserId};
use password_auth::{generate_hash, verify_password};
use redb::TableError::TableDoesNotExist;
use redb::{self, Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::collections::HashSet;
use std::hash::RandomState;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::info;
use types::{InternalError, Permission, Player, Role, UuidKey};
use uuid::Uuid;

const UUID_PLAYER: TableDefinition<UuidKey, Player> = TableDefinition::new("auth_uuid_player");
const NAME_UUID: TableDefinition<String, UuidKey> = TableDefinition::new("auth_player_name_uuid");
const UUID_ROLE: TableDefinition<UuidKey, Role> = TableDefinition::new("auth_uuid_role");

#[derive(Debug, Clone)]
pub struct Backend {
    pub db: Arc<Database>,
}

impl Backend {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn init(&self) -> Result<Self, InternalError> {
        let db = self.db.clone();
        let read_txn = db.begin_read()?;
        let uuid_role = read_txn.open_table(UUID_ROLE);
        // if role table doesn't exist insert admin user and role
        if let Err(TableDoesNotExist(_)) = uuid_role {
            let role_uuid = Uuid::new_v4();
            let admin_role = Role {
                uuid: role_uuid,
                name: "admin".to_string(),
                permissions: [Permission::AssignAdm, Permission::ChangeTargetBlock].into(),
            };
            self.insert_role(&admin_role).await?;
            let password_hash = generate_hash("aBcD123$");
            let mut roles = HashSet::new();
            roles.insert(role_uuid);
            let admin = Player {
                uuid: Uuid::new_v4(),
                name: "admin".to_string(),
                password_hash,
                permissions: Default::default(),
                roles,
            };
            self.insert_player(&admin).await?;
            info!("inserted admin_role and admin user");
        }
        Ok(Self { db })
    }

    pub async fn insert_player(&self, player: &Player) -> Result<Option<Player>, InternalError> {
        let db = self.db.clone();
        let player = player.clone();
        spawn_blocking(move || Backend::insert_player_blocking(db, player)).await?
    }

    pub fn insert_player_blocking(
        db: Arc<Database>,
        player: Player,
    ) -> Result<Option<Player>, InternalError> {
        let write_txn = db.begin_write()?;
        let player = {
            let mut uuid_player = &mut write_txn.open_table(UUID_PLAYER)?;
            let player_result = uuid_player
                .insert(&UuidKey(player.uuid.clone()), player.clone())
                .map(|opt| opt.map(|ag| ag.value()))
                .map_err(Into::into);
            let mut name_uuid = &mut write_txn.open_table(NAME_UUID)?;
            name_uuid
                .insert(&player.name, &UuidKey(player.uuid))
                .map_err(Into::<InternalError>::into)?;
            player_result
        };
        write_txn.commit().map(Into::into)?;
        player
    }

    pub async fn get_player_by_uuid(&self, uuid: &Uuid) -> Result<Option<Player>, InternalError> {
        let db = self.db.clone();
        let uuid_key = UuidKey(uuid.clone());
        spawn_blocking(move || Backend::get_player_by_uuid_blocking(db, uuid_key)).await?
    }

    fn get_player_by_uuid_blocking(
        db: Arc<Database>,
        uuid_key: UuidKey,
    ) -> Result<Option<Player>, InternalError> {
        let read_txn = db.begin_read()?;
        let uuid_player = read_txn.open_table(UUID_PLAYER)?;
        let opt_player = uuid_player.get(&uuid_key)?.map(|ag| ag.value());
        Ok(opt_player)
    }

    pub async fn get_player_by_name(&self, name: &String) -> Result<Option<Player>, InternalError> {
        let db = self.db.clone();
        let name = name.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read()?;
            let name_player_uuid = read_txn.open_table(NAME_UUID)?;
            let player = name_player_uuid
                .get(&name)?
                .map(|ag| ag.value().0)
                .map(|uuid| Backend::get_player_by_uuid_blocking(db, UuidKey(uuid)))
                .transpose()
                .map(|p| p.flatten())?;
            Ok::<Option<Player>, InternalError>(player)
        })
        .await?
    }

    pub async fn get_players(&self) -> Result<Vec<Player>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read()?;
            let uuid_player = read_txn.open_table(UUID_PLAYER)?;
            uuid_player
                .iter()?
                .map(|result| {
                    result
                        .map(|(_, player_ag)| player_ag.value())
                        .map_err(Into::into)
                })
                .collect::<Result<Vec<Player>, InternalError>>()
        })
        .await?
    }

    pub async fn insert_role(&self, role: &Role) -> Result<Option<Role>, InternalError> {
        let db = self.db.clone();
        let role = role.clone();
        spawn_blocking(move || Backend::insert_role_blocking(db, role)).await?
    }

    pub fn insert_role_blocking(
        db: Arc<Database>,
        role: Role,
    ) -> Result<Option<Role>, InternalError> {
        let uuid_key = UuidKey(role.uuid.clone());
        let write_txn = db.begin_write()?;
        let role = {
            let mut uuid_role = write_txn.open_table(UUID_ROLE)?;
            uuid_role
                .insert(&uuid_key, &role)
                .map(|opt| opt.map(|ag| ag.value()))
                .map_err(Into::into)
        };
        write_txn.commit().map(Into::into)?;
        role
    }

    pub async fn get_role_by_uuid(&self, uuid: &Uuid) -> Result<Option<Role>, InternalError> {
        let db = self.db.clone();
        let uuid_key = UuidKey(uuid.clone());
        spawn_blocking(move || Backend::get_role_by_uuid_blocking(db, uuid_key)).await?
    }

    fn get_role_by_uuid_blocking(
        db: Arc<Database>,
        uuid_key: UuidKey,
    ) -> Result<Option<Role>, InternalError> {
        let read_txn = db.begin_read()?;
        let name_role = read_txn.open_table(UUID_ROLE)?;
        name_role
            .get(uuid_key)
            .map(|opt| opt.map(|ag| ag.value()))
            .map_err(Into::into)
    }

    pub async fn get_roles(&self) -> Result<Vec<Role>, InternalError> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let read_txn = db.begin_read()?;
            let uuid_role = read_txn.open_table(UUID_ROLE)?;
            uuid_role
                .iter()?
                .map(|result| {
                    result
                        .map(|(_, role_ag)| role_ag.value())
                        .map_err(Into::into)
                })
                .collect::<Result<Vec<Role>, InternalError>>()
        })
        .await?
    }

    pub async fn get_roles_permissions(
        &self,
        roles: &HashSet<Uuid>,
    ) -> Result<HashSet<Permission, RandomState>, InternalError> {
        let db = self.db.clone();
        let roles = roles.clone();
        spawn_blocking(move || {
            let role_opts: Vec<Option<Role>> = roles
                .iter()
                .map(|uuid| UuidKey(uuid.clone()))
                .map(|uuid_key| Backend::get_role_by_uuid_blocking(db.clone(), uuid_key))
                .collect::<Result<Vec<Option<Role>>, InternalError>>()?;

            let permissions = role_opts
                .into_iter()
                .filter_map(|role_opt| role_opt.map(|role| role.permissions.clone().into_iter()))
                .flatten()
                .collect::<HashSet<Permission>>();
            Ok(permissions)
        })
        .await?
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<Backend>;

#[async_trait]
impl AuthnBackend for Backend {
    type User = Player;
    type Credentials = (String, String);
    type Error = InternalError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let (name, password) = creds;
        let uuid_player = self
            .get_player_by_name(&name)
            .await?
            .filter(|player| verify_password(password, player.password_hash.as_str()).is_ok());
        Ok(uuid_player)
    }

    async fn get_user(&self, uuid: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        self.get_player_by_uuid(uuid).await
    }
}

#[async_trait]
impl AuthzBackend for Backend {
    type Permission = Permission;

    async fn get_user_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        let mut permissions = HashSet::default();
        let player_opt = self.get_player_by_uuid(&user.uuid).await?;
        if let Some(player) = player_opt {
            permissions.extend(player.permissions);
        }
        Ok(permissions)
    }

    async fn get_group_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        let mut permissions = HashSet::default();
        let player_opt = self.get_player_by_uuid(&user.uuid).await?;
        if let Some(player) = player_opt {
            let player_role_permissions = self.get_roles_permissions(&player.roles).await?;
            permissions.extend(player_role_permissions);
        }
        Ok(permissions)
    }
}

#[cfg(test)]
mod test {
    use super::Backend;
    use crate::web::auth::types::{Permission, Player, Role};
    use password_auth::generate_hash;
    use redb::Database;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    fn temp_db() -> Arc<Database> {
        let file = NamedTempFile::new().unwrap().into_temp_path();
        Arc::new(Database::create(file).unwrap())
    }

    #[tokio::test]
    async fn test_insert_get_player() {
        let backend = Backend::new(temp_db());

        let input_password = "password";
        let password_hash = generate_hash(input_password);

        let inserted_player1 = Player {
            uuid: Uuid::new_v4(),
            name: "tester1".to_string(),
            password_hash: password_hash.clone(),
            permissions: [Permission::AssignAdm].into(),
            roles: Default::default(),
        };
        let existing_player1 = backend
            .insert_player(&inserted_player1)
            .await
            .expect("insert player1");
        assert_eq!(existing_player1, None);
        let get_player1 = backend
            .get_player_by_uuid(&inserted_player1.uuid)
            .await
            .expect("get player1 by uuid");
        assert_eq!(Some(inserted_player1.clone()), get_player1);
        let get_player1 = backend
            .get_player_by_name(&inserted_player1.name)
            .await
            .expect("get player1 by name");
        assert_eq!(Some(inserted_player1.clone()), get_player1);

        let inserted_player2 = Player {
            uuid: Uuid::new_v4(),
            name: "tester2".to_string(),
            password_hash,
            permissions: [Permission::AssignAdm].into(),
            roles: Default::default(),
        };
        let existing_player2 = backend
            .insert_player(&inserted_player2)
            .await
            .expect("insert player2");
        assert_eq!(existing_player2, None);

        let mut players = backend.get_players().await.expect("get players");
        players.sort_by(|p1, p2| p1.name.cmp(&p2.name));
        let mut inserted_players = vec![inserted_player1, inserted_player2];
        inserted_players.sort_by(|p1, p2| p1.name.cmp(&p2.name));
        assert_eq!(inserted_players, players);
    }

    #[tokio::test]
    async fn test_insert_get_role() {
        let backend = Backend::new(temp_db());
        let inserted_role1 = Role {
            uuid: Uuid::new_v4(),
            name: "test1".to_string(),
            permissions: [Permission::AssignAdm, Permission::ChangeTargetBlock].into(),
        };
        let existing_role1 = backend
            .insert_role(&inserted_role1)
            .await
            .expect("insert role1");
        assert_eq!(existing_role1, None);
        let get_role1 = backend
            .get_role_by_uuid(&inserted_role1.uuid)
            .await
            .expect("get role1 by uuid");
        assert_eq!(Some(inserted_role1.clone()), get_role1);

        let inserted_role2 = Role {
            uuid: Uuid::new_v4(),
            name: "test2".to_string(),
            permissions: [Permission::ChangeTargetBlock].into(),
        };
        let existing_role2 = backend
            .insert_role(&inserted_role2)
            .await
            .expect("insert role2");
        assert_eq!(existing_role2, None);

        let mut roles = backend.get_roles().await.expect("get roles");
        roles.sort_by(|p1, p2| p1.name.cmp(&p2.name));
        let mut inserted_roles = vec![inserted_role1.clone(), inserted_role2.clone()];
        inserted_roles.sort_by(|p1, p2| p1.name.cmp(&p2.name));
        assert_eq!(inserted_roles, roles);

        let inserted_role_uuids = HashSet::from_iter([inserted_role1.uuid, inserted_role2.uuid]);
        let permissions = backend
            .get_roles_permissions(&inserted_role_uuids)
            .await
            .expect("get roles permissions");
        let inserted_permissions =
            HashSet::from_iter([Permission::AssignAdm, Permission::ChangeTargetBlock]);
        assert_eq!(inserted_permissions, permissions);
    }
}
