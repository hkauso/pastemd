//! `model` manages the CRUD loop for pastes
use crate::{database::ClientManager, utility::unix_timestamp};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Paste {
    id: u32,
    url: String,
    content: String,
    password: String,
    date_published: u64,
    date_edited: u64,
}

// This is only needed when using Arc<Mutex<Vec<Paste>>>
// It only exists so we can do `paste[field]``
impl std::ops::Index<String> for Paste {
    type Output = String;

    fn index(&self, index: String) -> &Self::Output {
        match index.as_ref() {
            "url" => &self.url,
            _ => todo!(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteCreate {
    url: String,
    content: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteDelete {
    pub(super) password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteReturn {
    url: String,
    content: String,
    date_published: u64,
    date_edited: u64,
}

pub enum PasteError {
    PasswordIncorrect,
    AlreadyExists,
    NotFound,
    Other,
}

impl IntoResponse for PasteError {
    fn into_response(self) -> Response {
        use crate::model::PasteError::*;
        match self {
            PasswordIncorrect => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The given password is invalid.",
            )
                .into_response(),
            AlreadyExists => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A paste with this URL already exists.",
            )
                .into_response(),
            NotFound => (
                StatusCode::NOT_FOUND,
                "No paste with this URL has been found.",
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unspecified error occured with the paste manager",
            )
                .into_response(),
        }
    }
}

#[derive(Clone)]
pub struct PasteManager {
    // This will eventually be a lot more elaborate as a database is implemented, currently this mock storage is here so I can test the API
    manager: ClientManager<Paste>,
}

/// CRUD manager for pastes
///
/// TODO: use an actual database instead of in-memory `Arc<Mutex<Vec<Paste>>>`
impl PasteManager {
    /// Returns a new instance of `PasteManager`
    pub async fn init() -> Self {
        Self {
            manager: ClientManager::new(Arc::default()),
        }
    }

    /// Creates a new `Paste` from the input `PasteCreate`
    ///
    /// **Returns:** `Result<(), PasteError>`
    pub async fn create_paste(&self, paste: PasteCreate) -> Result<(), PasteError> {
        // make sure paste doesn't already exist
        if let Ok(_) = self.manager.select_single(String::from("url"), &paste.url) {
            return Err(PasteError::AlreadyExists);
        };

        // push
        let id = self.manager.len() as u32;

        match self.manager.insert_row(Paste {
            id, // Eventually this should come from the DB's unique ID
            url: paste.url,
            content: paste.content,
            password: paste.password,
            date_published: unix_timestamp(),
            date_edited: unix_timestamp(),
        }) {
            Ok(_) => Ok(()),
            Err(_) => Err(PasteError::Other),
        }
    }

    /// Retrieves a `Paste` from `PasteManager` by its `url`
    ///
    /// **Returns:** `Option<PasteReturn>`, where `None` signifies that the paste has not been found
    pub async fn get_paste_by_url(&self, paste_url: String) -> Result<PasteReturn, PasteError> {
        let searched_paste = self.manager.select_single(String::from("url"), &paste_url);

        match searched_paste {
            Ok(p) => Ok(PasteReturn {
                url: p.url.to_owned(),
                content: p.content.to_owned(),
                date_published: p.date_published,
                date_edited: p.date_edited,
            }),
            Err(_) => Err(PasteError::NotFound),
        }
    }

    /// Removes a `Paste` from `PasteManager` by its `url`
    ///
    /// **Returns:** `Option<PasteReturn>`, where `None` signifies that the paste has not been found
    pub async fn delete_paste_by_url(
        &self,
        paste_url: String,
        password: String,
    ) -> Result<(), PasteError> {
        // make sure paste exists
        let existing = match self.manager.select_single(String::from("url"), &paste_url) {
            Ok(p) => p,
            Err(_) => return Err(PasteError::NotFound),
        };

        // check password
        // in the future, hashes should be compared here
        if password != existing.password {
            return Err(PasteError::PasswordIncorrect);
        }

        // return
        self.manager.remove_single(String::from("url"), &paste_url)
    }
}
