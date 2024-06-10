//! `model` manages the CRUD loop for pastes
use crate::database::Database;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Paste {
    pub id: String,
    pub url: String,
    pub content: String,
    pub password: String,
    pub date_published: u64,
    pub date_edited: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteCreate {
    pub url: String,
    pub content: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteDelete {
    pub password: String,
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
            PasswordIncorrect => {
                (StatusCode::BAD_REQUEST, "The given password is invalid.").into_response()
            }
            AlreadyExists => (
                StatusCode::BAD_REQUEST,
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
    db: Database,
}

/// CRUD manager for pastes
///
/// TODO: use an actual database instead of in-memory `Arc<Mutex<Vec<Paste>>>`
impl PasteManager {
    /// Returns a new instance of `PasteManager`
    pub async fn init(opts: dorsal::DatabaseOpts) -> Self {
        let db = Database::new(opts).await;
        db.init().await;
        Self { db }
    }

    /// Creates a new `Paste` from the input `PasteCreate`
    ///
    /// **Returns:** `Result<(), PasteError>`
    pub async fn create_paste(&self, paste: PasteCreate) -> Result<(), PasteError> {
        self.db.create_paste(paste).await
    }

    /// Retrieves a `Paste` from `PasteManager` by its `url`
    ///
    /// **Returns:** `Option<PasteReturn>`, where `None` signifies that the paste has not been found
    pub async fn get_paste_by_url(&self, paste_url: String) -> Result<Paste, PasteError> {
        self.db.get_paste_by_url(paste_url).await
    }

    /// Removes a `Paste` from `PasteManager` by its `url`
    ///
    /// **Returns:** `Option<PasteReturn>`, where `None` signifies that the paste has not been found
    pub async fn delete_paste_by_url(
        &self,
        paste_url: String,
        password: String,
    ) -> Result<(), PasteError> {
        self.db.delete_paste(paste_url, password).await
    }
}
