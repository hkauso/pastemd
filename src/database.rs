use crate::model::{PasteCreate, PasteError, Paste, PasteMetadata, Document, DocumentCreate};

use dorsal::utility;
use dorsal::query as sqlquery;
use dorsal::db::special::auth_db::{FullUser, UserMetadata};
use serde::{Serialize, de::DeserializeOwned};

pub type Result<T> = std::result::Result<T, PasteError>;

#[derive(Clone, Debug, PartialEq)]
pub enum ViewMode {
    /// Only authenticated users can count as a paste view and only once
    AuthenticatedOnce,
    /// Anybody can count as a paste view multiple times;
    /// views are only stored in redis when using this mode
    OpenMultiple,
}

#[derive(Clone, Debug)]
pub struct ServerOptions {
    /// If pastes can require a password to be viewed
    pub view_password: bool,
    /// If authentication through guppy is enabled
    pub guppy: bool,
    /// If pastes can have a owner username (guppy required)
    pub paste_ownership: bool,
    /// If [`Document`]s are allowed (needed for external plugins)
    pub document_store: bool,
    /// View mode options
    pub view_mode: ViewMode,
}

impl ServerOptions {
    /// Enable all options
    pub fn truthy() -> Self {
        Self {
            view_password: true,
            guppy: true,
            paste_ownership: true,
            document_store: true,
            view_mode: ViewMode::OpenMultiple,
        }
    }
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            view_password: false,
            guppy: false,
            paste_ownership: false,
            document_store: false,
            view_mode: ViewMode::OpenMultiple,
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

        if self.options.view_mode == ViewMode::AuthenticatedOnce {
            // create table to track views
            let _ = sqlquery(
                "CREATE TABLE IF NOT EXISTS \"se_views\" (
                    url      TEXT,
                    username TEXT
                )",
            )
            .execute(c)
            .await;
        }

        if self.options.document_store == true {
            // create table to store documents
            let _ = sqlquery(
                "CREATE TABLE IF NOT EXISTS \"se_documents\" (
                    id        TEXT,
                    namespace TEXT,
                    content   TEXT,
                    timestamp TEXT,
                    metadata  TEXT
                )",
            )
            .execute(c)
            .await;
        }
    }

    // ...

    /// Get an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `url` - [`String`] of the paste's `url` field
    pub async fn get_paste_by_url(&self, mut url: String) -> Result<Paste> {
        url = idna::punycode::encode_str(&url).unwrap().to_lowercase();

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

    /// Create a new paste
    ///
    /// ## Arguments:
    /// * `props` - [`PasteCreate`]
    ///
    /// ## Returns:
    /// * Result containing a tuple with the unhashed edit password and the paste
    pub async fn create_paste(&self, mut props: PasteCreate) -> Result<(String, Paste)> {
        props.url = idna::punycode::encode_str(&props.url)
            .unwrap()
            .to_lowercase();

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
            .bind::<&String>(match serde_json::to_string(&paste.metadata) {
                Ok(ref s) => s,
                Err(_) => return Err(PasteError::ValueError),
            })
            .execute(c)
            .await
        {
            Ok(_) => return Ok((props.password, paste)),
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Delete an existing paste by `url`
    ///
    /// ## Arguments:
    /// * `url` - the paste to delete
    /// * `password` - the paste's edit password
    pub async fn delete_paste_by_url(&self, mut url: String, password: String) -> Result<()> {
        url = idna::punycode::encode_str(&url).unwrap().to_lowercase();

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

                if self.options.view_mode == ViewMode::AuthenticatedOnce {
                    // delete all view logs
                    let query: &str =
                        if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
                            "DELETE FROM \"se_views\" WHERE \"url\" = ?"
                        } else {
                            "DELETE FROM \"se_views\" WHERE \"url\" = $1"
                        };

                    if let Err(_) = sqlquery(query).bind::<&String>(&url).execute(c).await {
                        return Err(PasteError::Other);
                    };
                }

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
        url = idna::punycode::encode_str(&url).unwrap().to_lowercase();

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
        url = idna::punycode::encode_str(&url).unwrap().to_lowercase();

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
        url = idna::punycode::encode_str(&url).unwrap().to_lowercase();

        if url.ends_with("-") {
            url.pop();
        }

        // get views
        match self.base.cachedb.get(format!("se_views:{}", url)).await {
            Some(c) => c.parse::<i32>().unwrap(),
            None => {
                // try to count from "se_views"
                if self.options.view_mode == ViewMode::AuthenticatedOnce {
                    let query: &str =
                        if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
                            "SELECT * FROM \"se_views\" WHERE \"url\" = ?"
                        } else {
                            "SELECT * FROM \"se_views\" WHERE \"url\" = $1"
                        };

                    let c = &self.base.db.client;
                    match sqlquery(query).bind::<&String>(&url).fetch_all(c).await {
                        Ok(views) => {
                            let views = views.len();

                            // store in cache
                            self.base
                                .cachedb
                                .set(format!("se_views:{}", url), views.to_string())
                                .await;

                            // return
                            return views as i32;
                        }
                        Err(_) => return 0,
                    };
                }

                // return 0 by default
                0
            }
        }
    }

    /// Update an existing url's view count
    ///
    /// ## Arguments:
    /// * `url` - the paste to count the view for
    /// * `as_user` - the userstate of the user viewing this (for [`ViewMode::AuthenticatedOnce`])
    pub async fn incr_views_by_url(
        &self,
        mut url: String,
        as_user: Option<FullUser<UserMetadata>>,
    ) -> Result<()> {
        url = idna::punycode::encode_str(&url).unwrap().to_lowercase();

        if url.ends_with("-") {
            url.pop();
        }

        // handle AuthenticatedOnce
        if self.options.view_mode == ViewMode::AuthenticatedOnce {
            match as_user {
                Some(ua) => {
                    // check for view
                    if self
                        .user_has_viewed_paste(url.clone(), ua.user.username.clone())
                        .await
                    {
                        // can only view once in this mode
                        return Ok(());
                    }

                    // create view
                    let query: &str =
                        if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
                            "INSERT INTO \"se_views\" VALUES (?, ?)"
                        } else {
                            "INSERT INTO \"se_views\" VALEUS ($1, $2)"
                        };

                    let c = &self.base.db.client;
                    match sqlquery(query)
                        .bind::<&String>(&url)
                        .bind::<&String>(&ua.user.username)
                        .execute(c)
                        .await
                    {
                        Ok(_) => (), // do nothing so cache is incremented
                        Err(_) => return Err(PasteError::Other),
                    };
                }
                None => return Ok(()), // not technically an error, just not allowed
            }
        }

        // add view
        // views never reach the database, they're only stored in memory
        match self.base.cachedb.incr(format!("se_views:{}", url)).await {
            // swapped for some reason??
            false => Ok(()),
            true => Err(PasteError::Other),
        }
    }

    /// Check if a user has views a paste given the `url` and their `username`
    ///
    /// ## Arguments:
    /// * `url` - the paste url
    /// * `username` - the username of the user
    pub async fn user_has_viewed_paste(&self, url: String, username: String) -> bool {
        if self.options.view_mode == ViewMode::AuthenticatedOnce {
            let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql")
            {
                "SELECT * FROM \"se_views\" WHERE \"url\" = ? AND \"username\" = ?"
            } else {
                "SELECT * FROM \"se_views\" WHERE \"url\" = $1 AND \"username\" = ?"
            };

            let c = &self.base.db.client;
            match sqlquery(query)
                .bind::<&String>(&url)
                .bind::<&String>(&username)
                .fetch_one(c)
                .await
            {
                Ok(_) => return true,
                Err(_) => return false,
            };
        }

        false
    }

    // documents

    /// Pull an existing document by `id`
    ///
    /// ## Arguments:
    /// * `id` - [`String`] of the document's `id` field
    /// * `namespace` - [`String`] of the namespace the document belongs to
    pub async fn pull<
        T: Serialize + DeserializeOwned + From<String>,
        M: Serialize + DeserializeOwned,
    >(
        &self,
        id: String,
        namespace: String,
    ) -> Result<Document<T, M>> {
        if self.options.document_store == false {
            return Err(PasteError::Other);
        }

        // check in cache
        match self.base.cachedb.get(format!("se_document:{}", id)).await {
            Some(c) => return Ok(serde_json::from_str::<Document<T, M>>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"se_documents\" WHERE \"id\" = ? AND \"namespace\" = ?"
        } else {
            "SELECT * FROM \"se_documents\" WHERE \"id\" = $1 AND \"namespace\" = $2"
        };

        let c = &self.base.db.client;
        let res = match sqlquery(query)
            .bind::<&String>(&id)
            .bind::<&String>(&namespace)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).data,
            Err(_) => return Err(PasteError::NotFound),
        };

        // return
        let doc = Document {
            id: res.get("id").unwrap().to_string(),
            namespace: res.get("namespace").unwrap().to_string(),
            content: res.get("content").unwrap().to_string().into(),
            timestamp: res.get("date_published").unwrap().parse::<u128>().unwrap(),
            metadata: match serde_json::from_str(res.get("metadata").unwrap()) {
                Ok(m) => m,
                Err(_) => return Err(PasteError::ValueError),
            },
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("se_document:{}:{}", namespace, id),
                serde_json::to_string::<Document<T, M>>(&doc).unwrap(),
            )
            .await;

        // return
        Ok(doc)
    }

    /// Create a a new document
    ///
    /// Making sure values are unique should be done before calling `push`.
    ///
    /// ## Arguments:
    /// * `props` - [`DocumentCreate`]
    ///
    /// ## Returns:
    /// * Full [`Document`]
    pub async fn push<T: ToString, M: Serialize>(
        &self,
        props: DocumentCreate<T, M>,
    ) -> Result<Document<T, M>> {
        if self.options.document_store == false {
            return Err(PasteError::Other);
        }

        // ...
        let doc = Document {
            id: utility::random_id(),
            namespace: props.namespace,
            content: props.content,
            timestamp: utility::unix_epoch_timestamp(),
            metadata: props.metadata,
        };

        // create paste
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "INSERT INTO \"se_documents\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"se_documents\" VALEUS ($1, $2, $3, $4, $5)"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&doc.id)
            .bind::<&String>(&doc.namespace)
            .bind::<&String>(&doc.content.to_string())
            .bind::<&String>(&doc.timestamp.to_string())
            .bind::<&String>(match serde_json::to_string(&doc.metadata) {
                Ok(ref s) => s,
                Err(_) => return Err(PasteError::ValueError),
            })
            .execute(c)
            .await
        {
            Ok(_) => return Ok(doc),
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Delete an existing document by `id`
    ///
    /// Permission checks should be done before calling `drop`.
    ///
    /// ## Arguments:
    /// * `id` - the document to delete
    /// * `namespace` - the namespace the document belongs to
    pub async fn drop<
        T: Serialize + DeserializeOwned + From<String>,
        M: Serialize + DeserializeOwned,
    >(
        &self,
        id: String,
        namespace: String,
    ) -> Result<()> {
        if self.options.document_store == false {
            return Err(PasteError::Other);
        }

        // make sure document exists
        if let Err(e) = self.pull::<T, M>(id.clone(), namespace.clone()).await {
            return Err(e);
        };

        // delete document
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "DELETE FROM \"se_documents\" WHERE \"id\" = ? AND \"namespace\" = ?"
        } else {
            "DELETE FROM \"se_documents\" WHERE \"id\" = $1 AND \"namespace\" = $2"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&id)
            .bind::<&String>(&namespace)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("se_document:{}:{}", namespace, id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Edit an existing document by `id`
    ///
    /// Permission checks should be done before calling `update`.
    ///
    /// ## Arguments:
    /// * `id` - the document to edit
    /// * `namespace` - the namespace the document belongs to
    /// * `new_content` - the new content of the paste
    pub async fn update<
        T: Serialize + DeserializeOwned + From<String> + ToString,
        M: Serialize + DeserializeOwned,
    >(
        &self,
        id: String,
        namespace: String,
        new_content: String,
    ) -> Result<()> {
        if self.options.document_store == false {
            return Err(PasteError::Other);
        }

        // make sure document exists
        if let Err(e) = self.pull::<T, M>(id.clone(), namespace.clone()).await {
            return Err(e);
        };

        // edit document
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "UPDATE \"se_pastes\" SET \"content\" = ? WHERE \"url\" = ? AND \"namespace\" = ?"
        } else {
            "UPDATE \"se_pastes\" SET \"content\" = $1 WHERE \"url\" = $2 AND \"namespace\" = $3"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&new_content.to_string())
            .bind::<&String>(&id)
            .bind::<&String>(&namespace)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("se_document:{}:{}", namespace, id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(PasteError::Other),
        };
    }

    /// Edit an existing paste's metadata by `url`
    ///
    /// Permission checks should be done before calling `update`.
    ///
    /// ## Arguments:
    /// * `id` - the document to edit
    /// * `namespace` - the namespace the document belongs to    
    /// * `metadata` - the new metadata of the document
    pub async fn update_metadata<
        T: Serialize + DeserializeOwned + From<String> + ToString,
        M: Serialize + DeserializeOwned,
    >(
        &self,
        id: String,
        namespace: String,
        metadata: PasteMetadata,
    ) -> Result<()> {
        if self.options.document_store == false {
            return Err(PasteError::Other);
        }

        // make sure document exists
        if let Err(e) = self.pull::<T, M>(id.clone(), namespace.clone()).await {
            return Err(e);
        };

        // edit document
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "UPDATE \"se_documents\" SET \"metadata\" = ? WHERE \"url\" = ? AND \"namespace\" = ?"
        } else {
            "UPDATE \"se_documents\" SET \"metadata\" = $1 WHERE \"url\" = $2 AND \"namespace\" = $3"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(match serde_json::to_string(&metadata) {
                Ok(ref m) => m,
                Err(_) => return Err(PasteError::ValueError),
            })
            .bind::<&String>(&id)
            .bind::<&String>(&namespace)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("se_document:{}:{}", namespace, id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(PasteError::Other),
        };
    }
}
