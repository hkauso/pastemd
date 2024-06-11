//! Responds to API requests
use crate::model::{PasteCreate, PasteDelete, PasteEdit, PasteError, Paste};
use crate::database::Database;
use dorsal::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/new", post(create_paste))
        .route("/:url", get(get_paste_by_url))
        .route("/:url/delete", post(delete_paste_by_url))
        .route("/:url/edit", post(edit_paste_by_url))
        .with_state(database)
}

/// Create a new paste (`/api/new`)
async fn create_paste(
    State(database): State<Database>,
    Json(paste_to_create): Json<PasteCreate>,
) -> Result<Json<DefaultReturn<Paste>>, PasteError> {
    let res = database.create_paste(paste_to_create).await;

    match res {
        Ok(paste) => Ok(Json(DefaultReturn {
            success: true,
            message: String::from("Paste created"),
            payload: paste,
        })),
        Err(e) => Err(e),
    }
}

/// Delete an existing paste (`/api/:url/delete`)
async fn delete_paste_by_url(
    State(database): State<Database>,
    Path(url): Path<String>,
    Json(paste_to_delete): Json<PasteDelete>,
) -> Result<Json<DefaultReturn<()>>, PasteError> {
    match database
        .delete_paste_by_url(url, paste_to_delete.password)
        .await
    {
        Ok(_) => Ok(Json(DefaultReturn {
            success: true,
            message: String::from("Paste deleted"),
            payload: (),
        })),
        Err(e) => Err(e),
    }
}

/// Edit an existing paste (`/api/:url/edit`)
async fn edit_paste_by_url(
    State(database): State<Database>,
    Path(url): Path<String>,
    Json(paste_to_edit): Json<PasteEdit>,
) -> Result<Json<DefaultReturn<()>>, PasteError> {
    match database
        .edit_paste_by_url(
            url,
            paste_to_edit.password,
            paste_to_edit.new_content,
            paste_to_edit.new_url,
            paste_to_edit.new_password,
        )
        .await
    {
        Ok(_) => Ok(Json(DefaultReturn {
            success: true,
            message: String::from("Paste updated"),
            payload: (),
        })),
        Err(e) => Err(e),
    }
}

/// Get an existing paste by url (`/api/:url`)
pub async fn get_paste_by_url(
    State(database): State<Database>,
    Path(url): Path<String>,
) -> Result<Json<DefaultReturn<Paste>>, PasteError> {
    match database.get_paste_by_url(url).await {
        Ok(p) => Ok(Json(DefaultReturn {
            success: true,
            message: String::from("Paste exists"),
            payload: p,
        })),
        Err(e) => Err(e),
    }
}

// general
pub async fn not_found() -> impl IntoResponse {
    Json(DefaultReturn::<u16> {
        success: false,
        message: String::from("Path does not exist"),
        payload: 404,
    })
}
