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
    #[serde(default)]
    pub new_password: String,
    #[serde(default)]
    pub new_url: String,
}

/// General API errors
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
                StatusCode::UNAUTHORIZED,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: String::from("The given password is invalid."),
                    payload: 401,
                }),
            )
                .into_response(),
            AlreadyExists => (
                StatusCode::BAD_REQUEST,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: String::from("A paste with this URL already exists."),
                    payload: 400,
                }),
            )
                .into_response(),
            NotFound => (
                StatusCode::NOT_FOUND,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: String::from("No paste with this URL has been found."),
                    payload: 404,
                }),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: String::from("An unspecified error has occured"),
                    payload: 500,
                }),
            )
                .into_response(),
        }
    }
}
