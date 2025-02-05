use super::db::AuthDb;
use super::types::{Permission, Player, Role};
use crate::types::{InternalError, UuidKey};
use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, AuthzBackend, UserId};
use password_auth::verify_password;
use redb::Database;
use std::collections::HashSet;
use std::hash::RandomState;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthBackend {
    pub auth_db: AuthDb,
}

impl AuthBackend {
    pub fn new(database: Arc<Database>) -> Result<Self, InternalError> {
        let auth_db = AuthDb::new(database)?;
        Ok(Self { auth_db })
    }

    pub async fn insert_player(&self, player: &Player) -> Result<Option<Player>, InternalError> {
        let auth_db = self.auth_db.clone();
        let player = player.clone();
        spawn_blocking(move || {
            let mut write_txn = auth_db.begin_write()?;
            let insert_player_result = AuthDb::insert_player(&mut write_txn, player);
            write_txn.commit()?;
            insert_player_result
        })
        .await?
    }

    pub async fn get_player_by_uuid(&self, uuid: &Uuid) -> Result<Option<Player>, InternalError> {
        let auth_db = self.auth_db.clone();
        let uuid_key = UuidKey(*uuid);
        spawn_blocking(move || {
            let read_txn = auth_db.begin_read()?;
            AuthDb::get_player_by_uuid(&read_txn, uuid_key)
        })
        .await?
    }

    pub async fn get_player_by_name(&self, name: &str) -> Result<Option<Player>, InternalError> {
        let auth_db = self.auth_db.clone();
        let name = name.to_owned();
        spawn_blocking(move || {
            // let read_txn = db.begin_read()?;
            // let name_player_uuid = read_txn.open_table(NAME_UUID)?;
            // let player = name_player_uuid
            //     .get(&name)?
            //     .map(|ag| ag.value().0)
            //     .map(|uuid| Self::get_player_by_uuid_blocking(&read_txn, UuidKey(uuid)))
            //     .transpose()
            //     .map(|p| p.flatten())?;
            let read_txn = auth_db.begin_read()?;
            AuthDb::get_player_by_name(&read_txn, &name)
        })
        .await?
    }

    pub async fn get_players(&self) -> Result<Vec<Player>, InternalError> {
        let auth_db = self.auth_db.clone();
        spawn_blocking(move || {
            let read_txn = auth_db.begin_read()?;
            AuthDb::get_players(&read_txn)
        })
        .await?
    }

    pub async fn get_player_permissions(
        &self,
        player: &Player,
    ) -> Result<HashSet<Permission>, InternalError> {
        let mut permissions = player.permissions.clone();
        let roles_permissions = self.get_roles_permissions(&player.roles).await?;
        permissions.extend(roles_permissions);
        Ok(permissions)
    }

    pub async fn insert_role(&self, role: &Role) -> Result<Option<Role>, InternalError> {
        let auth_db = self.auth_db.clone();
        let role = role.clone();
        spawn_blocking(move || {
            let mut write_txn = auth_db.begin_write()?;
            let insert_role_result = AuthDb::insert_role(&mut write_txn, role);
            write_txn.commit()?;
            insert_role_result
        })
        .await?
    }

    pub async fn get_role_by_uuid(&self, uuid: &Uuid) -> Result<Option<Role>, InternalError> {
        let auth_db = self.auth_db.clone();
        let uuid_key = UuidKey(*uuid);
        spawn_blocking(move || {
            let read_txn = auth_db.begin_read()?;
            AuthDb::get_role_by_uuid(&read_txn, uuid_key)
        })
        .await?
    }

    pub async fn get_roles(&self) -> Result<Vec<Role>, InternalError> {
        let auth_db = self.auth_db.clone();
        spawn_blocking(move || {
            let read_txn = auth_db.begin_read()?;
            AuthDb::get_roles(&read_txn)
        })
        .await?
    }

    pub async fn get_roles_permissions(
        &self,
        roles: &HashSet<Uuid>,
    ) -> Result<HashSet<Permission, RandomState>, InternalError> {
        let auth_db = self.auth_db.clone();
        let roles = roles.clone();
        spawn_blocking(move || {
            let read_txn = auth_db.begin_read()?;
            let role_opts: Vec<Option<Role>> = roles
                .iter()
                .map(|uuid| UuidKey(*uuid))
                .map(|uuid_key| AuthDb::get_role_by_uuid(&read_txn, uuid_key))
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
pub type AuthSession = axum_login::AuthSession<AuthBackend>;

#[async_trait]
impl AuthnBackend for AuthBackend {
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
impl AuthzBackend for AuthBackend {
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

impl AuthUser for Player {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.uuid
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::AuthBackend;
    use crate::auth::types::{Permission, Player, Role};
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
        let backend = AuthBackend::new(temp_db()).expect("new backend");

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
        assert_eq!(inserted_players, players[1..]);
    }

    #[tokio::test]
    async fn test_insert_get_role() {
        let backend = AuthBackend::new(temp_db()).expect("new backend");
        let inserted_role1 = Role {
            uuid: Uuid::new_v4(),
            name: "test1".to_string(),
            permissions: [Permission::AssignAdm, Permission::ChangeTarget].into(),
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
            permissions: [Permission::ChangeTarget].into(),
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
        assert_eq!(inserted_roles, roles[1..]);

        let inserted_role_uuids = HashSet::from_iter([inserted_role1.uuid, inserted_role2.uuid]);
        let permissions = backend
            .get_roles_permissions(&inserted_role_uuids)
            .await
            .expect("get roles permissions");
        let mut permissions = permissions.into_iter().collect::<Vec<Permission>>();
        permissions.sort();
        let inserted_permissions = [Permission::AssignAdm, Permission::ChangeTarget];
        assert_eq!(inserted_permissions, permissions[..]);
    }
}
