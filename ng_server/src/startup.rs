use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{migrate, Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webauthn_rs::prelude::*;

/*
 * Webauthn RS server side app state and setup  code.
 */

// Configure the Webauthn instance by using the WebauthnBuilder. This defines
// the options needed for your site, and has some implications. One of these is that
// you can NOT change your rp_id (relying party id), without invalidating all
// webauthn credentials. Remember, rp_id is derived from your URL origin, meaning
// that it is your effective domain name.

#[derive(Debug, Clone)]
pub struct Data {
    pub pool: Pool<Sqlite>,
    pub name_to_id: HashMap<String, Uuid>,
    pub id_to_name: HashMap<Uuid, String>,
    pub keys: HashMap<Uuid, Vec<Passkey>>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    // Webauthn has no mutable inner state, so Arc and read only is sufficent.
    // Alternately, you could use a reference here provided you can work out
    // lifetimes.
    pub webauthn: Arc<Webauthn>,
    // This needs mutability, so does require a mutex.
    // pub users: Arc<Mutex<Data>>,
    // DB connection pool
    pub pool: Pool<Sqlite>,
}

impl AppState {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        // Effective domain name.
        let rp_id = "localhost";
        // Url containing the effective domain name
        // MUST include the port number!
        let rp_origin = Url::parse("http://localhost:8081").expect("Invalid URL");
        let builder = WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid configuration");

        // Now, with the builder you can define other options.
        // Set a "nice" relying party name. Has no security properties and
        // may be changed in the future.
        let builder = builder.rp_name("Axum Webauthn-rs");

        // Consume the builder and create our webauthn instance.
        let webauthn = Arc::new(builder.build().expect("Invalid configuration"));

        // let users = Arc::new(Mutex::new(Data {
        //     pool,
        //     name_to_id: HashMap::new(),
        //     id_to_name: HashMap::new(),
        //     keys: HashMap::new(),
        // }));

        AppState { webauthn, pool }
    }
}
