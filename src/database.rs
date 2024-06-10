use crate::model::{PasteCreate, PasteError, Paste};

use dorsal::utility;
use dorsal::query as sqlquery;

pub type Result<T> = std::result::Result<T, PasteError>;

/// Database connector
#[derive(Clone)]
pub struct Database {
    pub base: dorsal::StarterDatabase,
}

impl Database {
    pub async fn new(opts: dorsal::DatabaseOpts) -> Self {
        Self {
            base: dorsal::StarterDatabase::new(opts).await,
        }
    }

    /// Init database
    pub async fn init(&self) {
        // create tables
        let c = &self.base.db.client;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"pastes\" (
                 id             TEXT,
                 url            TEXT,
                 password       TEXT,
                 content        TEXT,
                 date_published TEXT,
                 date_edited    TEXT
             )",
        )
        .execute(c)
        .await;
    }

    // ...

    /// Get an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `url` - [`String`] of the paste's `url` field
    pub async fn get_paste_by_url(&self, url: String) -> Result<Paste> {
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"pastes\" WHERE \"url\" = ?"
        } else {
            "SELECT * FROM \"pastes\" WHERE \"url\" = $1"
        };

        let c = &self.base.db.client;
        let res = match sqlquery(query)
            .bind::<&String>(&url.to_lowercase())
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).data,
            Err(_) => return Err(PasteError::NotFound),
        };

        // return
        // TODO: cache original result (res)
        let paste = Paste {
            id: res.get("id").unwrap().to_string(),
            url: res.get("url").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            password: res.get("password").unwrap().to_string(),
            date_published: res.get("date_published").unwrap().parse::<u64>().unwrap(),
            date_edited: res.get("date_edited").unwrap().parse::<u64>().unwrap(),
        };

        Ok(paste)
    }

    /// Get an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `props` - [`PasteCreate`]   
    pub async fn create_paste(&self, props: PasteCreate) -> Result<()> {
        // make sure paste doesn't already exist
        if let Ok(_) = self.get_paste_by_url(props.url.clone()).await {
            return Err(PasteError::AlreadyExists);
        }

        // TODO: check url length, content length, etc

        // create paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "INSERT INTO \"pastes\" VALUES (?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"pastes\" VALEUS ($1, $2, $3, $4, $5, $6)"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&utility::random_id())
            .bind::<&String>(&props.url)
            .bind::<&String>(&utility::hash(props.password))
            .bind::<&String>(&props.content)
            .bind::<&String>(&crate::utility::unix_timestamp().to_string())
            .bind::<&String>(&crate::utility::unix_timestamp().to_string())
            .execute(c)
            .await
        {
            Ok(_) => return Ok(()),
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Get an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `url` - the paste to delete
    /// * `password` - the paste's edit password
    pub async fn delete_paste(&self, url: String, password: String) -> Result<()> {
        // get paste
        let existing = match self.get_paste_by_url(url.clone()).await {
            Ok(p) => p,
            Err(err) => return Err(err),
        };

        // check password
        if utility::hash(password) != existing.password {
            return Err(PasteError::PasswordIncorrect);
        }

        // create paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "DELETE FROM \"pastes\" WHERE \"url\" = ?"
        } else {
            "DELETE FROM \"pastes\" WHERE \"url\" = $1"
        };

        let c = &self.base.db.client;
        match sqlquery(query).bind::<&String>(&url).execute(c).await {
            Ok(_) => return Ok(()),
            Err(_) => return Err(PasteError::Other),
        };
    }
}
