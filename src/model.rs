//! `model` manages the CRUD loop for pastes
use std::sync::{Arc, Mutex};
use axum::{http::StatusCode, response::{IntoResponse, Response}};
use serde::{Serialize, Deserialize};
use crate::utility::unix_timestamp;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Paste {
    id:             u32,
    url:            String,
    content:        String,
    password:       String,
    date_published: u64,
    date_edited:    u64
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PasteCreate {
    url:            String,
    content:        String,
    password:       String
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PasteReturn {
    url:            String,
    content:        String,
    date_published: u64,
    date_edited:    u64
}
pub enum PasteError {
    AlreadyExists,
    NotFound,
    Other
}
impl IntoResponse for PasteError {
    fn into_response(self) -> Response {
        use crate::model::PasteError::*;
        match self {
            AlreadyExists =>
                (StatusCode::INTERNAL_SERVER_ERROR, "A paste with this URL already exists.").into_response(),
            NotFound =>
                (StatusCode::NOT_FOUND, "No paste with this URL has been found.").into_response(),
            _ => 
                (StatusCode::INTERNAL_SERVER_ERROR, "An unspecified error occured with the paste manager").into_response()
        }
    }
}

#[derive(Clone)]
pub struct PasteManager {
    // This will eventually be a lot more elaborate as a database is implemented, currently this mock storage is here so I can test the API
    pastes: Arc<Mutex<Vec<Paste>>>
}
/// CRUD manager for pastes
/// 
/// TODO: use an actual database instead of in-memory `Arc<Mutex<Vec<Paste>>>`
impl PasteManager {
    /// Returns a new instance of `PasteManager`
    pub async fn init() -> Self {
        Self {
            pastes: Arc::default()
        }
    }
    /// Creates a new `Paste` from the input `PasteCreate`
    /// 
    /// **Returns:** `Result<(), PasteError>`
    pub async fn create_paste(&self, paste: PasteCreate) -> Result<(), PasteError> {
        let mut store = self.pastes.lock().unwrap();
        if let Some(_) = store.iter().find(|p| p.url == paste.url) {
            return Err(PasteError::AlreadyExists)
        }
        let id = store.len() as u32;
        store.push(Paste{
            id,             // Eventually this should come from the DB's unique ID
            url:            paste.url,
            content:        paste.content,
            password:       paste.password,
            date_published: unix_timestamp(),
            date_edited:    unix_timestamp()
        });
        Ok(())
    }
    /// Retrieves a `Paste` from `PasteManager` by its `url`
    /// 
    /// **Returns:** `Option<PasteReturn>`, where `None` signifies that the paste has not been found
    pub async fn get_paste_by_url(&self, paste_url: String) -> Result<PasteReturn, PasteError> {
        let store = self.pastes.lock().unwrap();
        // let mut filtered: Vec<PasteReturn> = store
        //     .clone().iter()
        //     .filter(|paste| paste.url == paste_url)
        //     .map(|paste| PasteReturn { 
        //         url: paste.url.to_owned(),
        //         content: paste.content.to_owned(),
        //         date_published: paste.date_published,
        //         date_edited: paste.date_edited
        //     })
        //     .collect();
        let searched_paste = store.iter().find(|paste| paste.url == paste_url);
        match searched_paste {
            Some(p) => {
                Ok(PasteReturn {
                    url: p.url.to_owned(),
                    content: p.content.to_owned(),
                    date_published: p.date_published,
                    date_edited: p.date_edited
                })
            }
            None => Err(PasteError::NotFound)
        }
    }
}