//! `routing::api` responds to requests that should return serialized data to the client. It creates an interface for the `PasteManager` CRUD struct defined in `model`
use crate::model::{PasteCreate, PasteDelete, PasteEdit, PasteError, Paste};
use crate::database::Database;
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

async fn create_paste(
    State(database): State<Database>,
    Json(paste_to_create): Json<PasteCreate>,
) -> Result<(), PasteError> {
    let res = database.create_paste(paste_to_create).await;
    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn delete_paste_by_url(
    State(database): State<Database>,
    Path(url): Path<String>,
    Json(paste_to_delete): Json<PasteDelete>,
) -> Result<(), PasteError> {
    database
        .delete_paste_by_url(url, paste_to_delete.password)
        .await
}

async fn edit_paste_by_url(
    State(database): State<Database>,
    Path(url): Path<String>,
    Json(paste_to_edit): Json<PasteEdit>,
) -> Result<(), PasteError> {
    database
        .edit_paste_by_url(url, paste_to_edit.password, paste_to_edit.new_content)
        .await
}

pub async fn get_paste_by_url(
    State(database): State<Database>,
    Path(url): Path<String>,
) -> Result<Json<Paste>, PasteError> {
    let return_paste = database.get_paste_by_url(url).await;
    match return_paste {
        Ok(p) => Ok(Json(p)),
        Err(e) => Err(e),
    }
}
