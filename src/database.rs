use crate::model::PasteError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type Result<T> = std::result::Result<T, PasteError>;

/// Database connector
pub struct Database {}

impl Database {
    pub fn init() -> Connection {
        let conn = Connection::open("main.db").unwrap();
        let table_result = conn.execute(
            "create table if not exists pastes (
                 id             integer primary key,
                 url            text,
                 password       text,
                 content        text,
                 date_published text,
                 date_edited    text
             )",
            (),
        );
        match table_result {
            Ok(_) => println!("Successfully connected to table."),
            Err(e) => panic!("Database creation failed with error message: {e}"),
        };
        conn
    }
}

/// Client manager for the database
///
/// Currently this is unused, and will stay so until the model and api are finished and I can transition from the PasteManager's mock storage to an actual database
#[derive(Clone)]
pub struct ClientManager<T> {
    /// Temporary store, it's labeled "pool" but is not currently a pool
    pool: ConnectionPool<T>,
}

type ConnectionPool<T> = Arc<Mutex<Vec<T>>>;

impl<T: std::ops::Index<String, Output = String> + Clone> ClientManager<T> {
    /// Create new [`Client`]
    pub fn new(pool: ConnectionPool<T>) -> Self {
        Self { pool }
    }

    // This is not needed when replaced with SQL methods
    pub fn len(&self) -> usize {
        // obtain client from pool
        let client = self.pool.lock().unwrap();

        // return
        client.len()
    }

    // functions below should be altered when proper database support is added
    // we only really need to select one thing at a time since the api is pretty basic,
    // so these functions only do what is needed ... optionally this could all be replaced
    // with a `run_query` function (or something)

    /// Select by a given `field`
    ///
    /// ## Arguments:
    /// * `field` - the field we are selecting by
    /// * `equals` - what the field value needs to equal
    pub fn select_single(&self, field: String, equals: &str) -> Result<T> {
        // obtain client from pool
        let client = self.pool.lock().unwrap();

        // select
        // (replace with sql "SELECT FROM ... WHERE ... LIMIT 1", this just implements a basic version)
        let entry = client.iter().clone().find(|r| r[field.clone()] == equals);

        match entry {
            // we need T to impl Clone so we can do this
            Some(r) => Ok((*r).to_owned()),
            None => Err(PasteError::NotFound),
        }
    }

    /// Insert `T`
    ///
    /// ## Arguments:
    /// * `value`: `T`
    pub fn insert_row(&self, value: T) -> Result<()> {
        // obtain client from pool
        let mut client = self.pool.lock().unwrap();

        // push and return
        client.push(value);
        Ok(())
    }

    /// Remove row by `field`
    ///
    /// ## Arguments:
    /// * `field` - the field we are selecting by
    /// * `equals` - what the field value needs to equal
    pub fn remove_single(&self, field: String, equals: &str) -> Result<()> {
        // obtain client from pool
        let mut client = self.pool.lock().unwrap();

        // remove
        // (replace with sql "REMOVE FROM ... WHERE ... LIMIT 1", this just implements a basic version)

        // this is very bad and only for testing, it'll go through everything to find what we want
        for (i, row) in client.clone().iter().enumerate() {
            if row[field.clone()] != equals {
                continue;
            }

            client.remove(i);
            break;
        }

        // return
        Ok(())
    }
}
