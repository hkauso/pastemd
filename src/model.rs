use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use serde::{Deserialize, Serialize};
use dorsal::DefaultReturn;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Paste {
    pub id: String,
    pub url: String,
    pub content: String,
    pub password: String,
    pub date_published: u128,
    pub date_edited: u128,
    pub metadata: PasteMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PasteMetadata {}

impl Default for PasteMetadata {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteCreate {
    #[serde(default)]
    pub url: String,
    pub content: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteDelete {
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteEdit {
    pub password: String,
    pub new_content: String,
    #[serde(default)]
    pub new_password: String,
    #[serde(default)]
    pub new_url: String,
}

/// General API errors
pub enum PasteError {
    PasswordIncorrect,
    AlreadyExists,
    ValueError,
    NotFound,
    Other,
}

impl PasteError {
    pub fn to_string(&self) -> String {
        use crate::model::PasteError::*;
        match self {
            PasswordIncorrect => String::from("The given password is invalid."),
            AlreadyExists => String::from("A paste with this URL already exists."),
            ValueError => String::from("One of the field values given is invalid."),
            NotFound => String::from("No paste with this URL has been found."),
            _ => String::from("An unspecified error has occured"),
        }
    }
}

impl IntoResponse for PasteError {
    fn into_response(self) -> Response {
        use crate::model::PasteError::*;
        match self {
            PasswordIncorrect => (
                StatusCode::UNAUTHORIZED,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 401,
                }),
            )
                .into_response(),
            AlreadyExists => (
                StatusCode::BAD_REQUEST,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 400,
                }),
            )
                .into_response(),
            NotFound => (
                StatusCode::NOT_FOUND,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 404,
                }),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 500,
                }),
            )
                .into_response(),
        }
    }
}
