use crate::model::{PasteCreate, PasteError, Paste, PasteMetadata};

use dorsal::utility;
use dorsal::query as sqlquery;
use dorsal::db::special::auth_db::{FullUser, UserMetadata};

pub type Result<T> = std::result::Result<T, PasteError>;

#[derive(Clone, Debug)]
pub struct ServerOptions {
    /// If pastes can require a password to be viewed
    pub view_password: bool,
    /// If authentication through guppy is enabled
    pub guppy: bool,
    /// Paste owner username (guppy required)
    pub paste_ownership: bool,
}

impl ServerOptions {
    /// Enable all options
    pub fn truthy() -> Self {
        Self {
            view_password: true,
            guppy: true,
            paste_ownership: true,
        }
    }
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            view_password: false,
            guppy: false,
            paste_ownership: false,
        }
    }
}

/// Database connector
#[derive(Clone)]
pub struct Database {
    pub base: dorsal::StarterDatabase,
    pub auth: dorsal::AuthDatabase,
    pub options: ServerOptions,
}

impl Database {
    pub async fn new(opts: dorsal::DatabaseOpts, opts1: ServerOptions) -> Self {
        let base = dorsal::StarterDatabase::new(opts).await;

        Self {
            base: base.clone(),
            auth: dorsal::AuthDatabase::new(base).await,
            options: opts1,
        }
    }

    /// Init database
    pub async fn init(&self) {
        // create tables
        let c = &self.base.db.client;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"se_pastes\" (
                 id             TEXT,
                 url            TEXT,
                 password       TEXT,
                 content        TEXT,
                 date_published TEXT,
                 date_edited    TEXT,
                 metadata       TEXT
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
    pub async fn get_paste_by_url(&self, mut url: String) -> Result<Paste> {
        url = idna::punycode::encode_str(&url).unwrap();

        if url.ends_with("-") {
            url.pop();
        }

        // check in cache
        match self.base.cachedb.get(format!("se_paste:{}", url)).await {
            Some(c) => return Ok(serde_json::from_str::<Paste>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"se_pastes\" WHERE \"url\" = ?"
        } else {
            "SELECT * FROM \"se_pastes\" WHERE \"url\" = $1"
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
        let paste = Paste {
            id: res.get("id").unwrap().to_string(),
            url: res.get("url").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            password: res.get("password").unwrap().to_string(),
            date_published: res.get("date_published").unwrap().parse::<u128>().unwrap(),
            date_edited: res.get("date_edited").unwrap().parse::<u128>().unwrap(),
            metadata: match serde_json::from_str(res.get("metadata").unwrap()) {
                Ok(m) => m,
                Err(_) => return Err(PasteError::ValueError),
            },
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("se_paste:{}", url),
                serde_json::to_string::<Paste>(&paste).unwrap(),
            )
            .await;

        // return
        Ok(paste)
    }

    /// Get an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `props` - [`PasteCreate`]
    ///
    /// ## Returns:
    /// * Result containing a tuple with the unhashed edit password and the paste
    pub async fn create_paste(&self, mut props: PasteCreate) -> Result<(String, Paste)> {
        props.url = idna::punycode::encode_str(&props.url).unwrap();

        if props.url.ends_with("-") {
            props.url.pop();
        }

        // make sure paste doesn't already exist
        if let Ok(_) = self.get_paste_by_url(props.url.clone()).await {
            return Err(PasteError::AlreadyExists);
        }

        // create url if not supplied
        if props.url.is_empty() {
            props.url = utility::random_id().chars().take(10).collect();
        }

        // create random password if not supplied
        if props.password.is_empty() {
            props.password = utility::random_id().chars().take(10).collect();
        }

        // check lengths
        if (props.url.len() > 250) | (props.url.len() < 3) {
            return Err(PasteError::ValueError);
        }

        if (props.content.len() > 200_000) | (props.content.len() < 1) {
            return Err(PasteError::ValueError);
        }

        // (characters used)
        let regex = regex::RegexBuilder::new("^[\\w\\_\\-\\.\\!\\p{Extended_Pictographic}]+$")
            .multi_line(true)
            .build()
            .unwrap();

        if regex.captures(&props.url).iter().len() < 1 {
            return Err(PasteError::ValueError);
        }

        // ...
        let paste = Paste {
            id: utility::random_id(),
            url: props.url,
            content: props.content,
            password: utility::hash(props.password.clone()),
            date_published: utility::unix_epoch_timestamp(),
            date_edited: utility::unix_epoch_timestamp(),
            metadata: super::model::PasteMetadata::default(),
        };

        // create paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "INSERT INTO \"se_pastes\" VALUES (?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"se_pastes\" VALEUS ($1, $2, $3, $4, $5, $6, $7)"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&paste.id)
            .bind::<&String>(&paste.url)
            .bind::<&String>(&paste.password)
            .bind::<&String>(&paste.content)
            .bind::<&String>(&paste.date_published.to_string())
            .bind::<&String>(&paste.date_edited.to_string())
            .bind::<&String>(
                match serde_json::to_string(&super::model::PasteMetadata::default()) {
                    Ok(ref s) => s,
                    Err(_) => return Err(PasteError::ValueError),
                },
            )
            .execute(c)
            .await
        {
            Ok(_) => return Ok((props.password, paste)),
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Get an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `url` - the paste to delete
    /// * `password` - the paste's edit password
    pub async fn delete_paste_by_url(&self, mut url: String, password: String) -> Result<()> {
        url = idna::punycode::encode_str(&url).unwrap();

        if url.ends_with("-") {
            url.pop();
        }

        // get paste
        let existing = match self.get_paste_by_url(url.clone()).await {
            Ok(p) => p,
            Err(err) => return Err(err),
        };

        // check password
        if utility::hash(password) != existing.password {
            return Err(PasteError::PasswordIncorrect);
        }

        // delete paste view count
        self.base.cachedb.remove(format!("se_views:{}", url)).await;

        // delete paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "DELETE FROM \"se_pastes\" WHERE \"url\" = ?"
        } else {
            "DELETE FROM \"se_pastes\" WHERE \"url\" = $1"
        };

        let c = &self.base.db.client;
        match sqlquery(query).bind::<&String>(&url).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base.cachedb.remove(format!("se_paste:{}", url)).await;

                // return
                return Ok(());
            }
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Edit an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `url` - the paste to edit
    /// * `password` - the paste's edit password
    /// * `new_content` - the new content of the paste
    /// * `new_url` - the new url of the paste
    /// * `new_password` - the new password of the paste
    /// * `editing_as` - the userstate of the user we're editing the paste as
    pub async fn edit_paste_by_url(
        &self,
        mut url: String,
        password: String,
        new_content: String,
        mut new_url: String,
        mut new_password: String,
        editing_as: Option<FullUser<UserMetadata>>,
    ) -> Result<()> {
        url = idna::punycode::encode_str(&url).unwrap();

        if url.ends_with("-") {
            url.pop();
        }

        // get paste
        let existing = match self.get_paste_by_url(url.clone()).await {
            Ok(p) => p,
            Err(err) => return Err(err),
        };

        // check password
        let mut skip_password_check: bool = false;

        if let Some(ua) = editing_as {
            // check if we're the paste owner
            if ua.user.username == existing.metadata.owner {
                skip_password_check = true;
            }
            // check if we have the "ManagePastes" permission
            else if ua.level.permissions.contains(&"ManagePastes".to_string()) {
                skip_password_check = true;
            }
        }

        if skip_password_check == false {
            if utility::hash(password) != existing.password {
                return Err(PasteError::PasswordIncorrect);
            }
        }

        // hash new password
        if !new_password.is_empty() {
            new_password = utility::hash(new_password);
        } else {
            new_password = existing.password;
        }

        // update new_url
        if new_url.is_empty() {
            new_url = existing.url;
        }

        new_url = idna::punycode::encode_str(&new_url).unwrap();

        if new_url.ends_with("-") {
            new_url.pop();
        }

        // edit paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "UPDATE \"se_pastes\" SET \"content\" = ?, \"password\" = ?, \"url\" = ?, \"date_edited\" = ? WHERE \"url\" = ?"
        } else {
            "UPDATE \"se_pastes\" SET (\"content\" = $1, \"password\" = $2, \"url\" = $3, \"date_edited\" = $4) WHERE \"url\" = $5"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&new_content)
            .bind::<&String>(&new_password)
            .bind::<&String>(&new_url)
            .bind::<&String>(&utility::unix_epoch_timestamp().to_string())
            .bind::<&String>(&url)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base.cachedb.remove(format!("se_paste:{}", url)).await;

                // return
                return Ok(());
            }
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Edit an existing paste's metadata by `url`
    ///
    /// ## Arguments:
    /// * `url` - the paste to edit
    /// * `password` - the paste's edit password
    /// * `metadata` - the new metadata of the paste
    /// * `editing_as` - the userstate of the user we're editing the paste as
    pub async fn edit_paste_metadata_by_url(
        &self,
        mut url: String,
        password: String,
        metadata: PasteMetadata,
        editing_as: Option<FullUser<UserMetadata>>,
    ) -> Result<()> {
        url = idna::punycode::encode_str(&url).unwrap();

        if url.ends_with("-") {
            url.pop();
        }

        // get paste
        let existing = match self.get_paste_by_url(url.clone()).await {
            Ok(p) => p,
            Err(err) => return Err(err),
        };

        // check password
        let mut skip_password_check: bool = false;

        if let Some(ua) = editing_as {
            // check if we're the paste owner
            if ua.user.username == existing.metadata.owner {
                skip_password_check = true;
            }
            // check if we have the "ManagePastes" permission
            else if ua.level.permissions.contains(&"ManagePastes".to_string()) {
                skip_password_check = true;
            }
        }

        if skip_password_check == false {
            if utility::hash(password) != existing.password {
                return Err(PasteError::PasswordIncorrect);
            }
        }

        // edit paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "UPDATE \"se_pastes\" SET \"metadata\" = ? WHERE \"url\" = ?"
        } else {
            "UPDATE \"se_pastes\" SET (\"metadata\" = $1) WHERE \"url\" = $2"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(match serde_json::to_string(&metadata) {
                Ok(ref m) => m,
                Err(_) => return Err(PasteError::ValueError),
            })
            .bind::<&String>(&url)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base.cachedb.remove(format!("se_paste:{}", url)).await;

                // return
                return Ok(());
            }
            Err(_) => return Err(PasteError::Other),
        };
    }

    // views

    /// Get an existing url's view count
    ///
    /// ## Arguments:
    /// * `url` - the paste to count the view for
    pub async fn get_views_by_url(&self, mut url: String) -> i32 {
        url = idna::punycode::encode_str(&url).unwrap();

        if url.ends_with("-") {
            url.pop();
        }

        // get views
        match self.base.cachedb.get(format!("se_views:{}", url)).await {
            Some(c) => c.parse::<i32>().unwrap(),
            None => 0,
        }
    }

    /// Update an existing url's view count
    ///
    /// ## Arguments:
    /// * `url` - the paste to count the view for
    pub async fn incr_views_by_url(&self, mut url: String) -> Result<()> {
        url = idna::punycode::encode_str(&url).unwrap();

        if url.ends_with("-") {
            url.pop();
        }

        // add view
        // views never reach the database, they're only stored in memory
        match self.base.cachedb.incr(format!("se_views:{}", url)).await {
            // swapped for some reason??
            false => Ok(()),
            true => Err(PasteError::Other),
        }
    }
}
