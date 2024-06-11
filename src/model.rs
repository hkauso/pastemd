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

#[derive(Serialize, Deserialize, Debug)]
pub struct PasteEdit {
    pub password: String,
    pub new_content: String,
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
