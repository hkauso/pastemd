//! `routing::api` creates an interface for the `PasteManager` CRUD struct defined in `model`
use crate::model::{PasteCreate, PasteError, PasteManager, PasteReturn};
use axum::{
    extract::{Path, State}, routing::{get, post}, Json, Router,
};
use axum_macros::debug_handler;

pub fn routes(manager: PasteManager) -> Router {
    Router::new()
        .route("/new", post(create_paste))
        .route("/:url", get(get_paste_by_url))
        .with_state(manager)
}
async fn create_paste(
    State(manager): State<PasteManager>, 
    Json(paste_to_create): Json<PasteCreate>
) -> Result<(), PasteError> {
    manager.create_paste(paste_to_create).await
}
#[debug_handler]
async fn get_paste_by_url(
    State(manager): State<PasteManager>,
    Path(url): Path<String>
) -> Result<Json<PasteReturn>, PasteError> {
    let return_paste = manager.get_paste_by_url(url).await;
    match return_paste {
        Ok(p)  => Ok(Json(p)),
        Err(e) => Err(e)
    }
}