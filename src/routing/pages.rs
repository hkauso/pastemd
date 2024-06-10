use askama_axum::Template;
use axum::{extract::{Path, State}, response::{IntoResponse, Html}, routing::get, Router};
use crate::model::{self, PasteError, PasteManager, PasteReturn};

// `routing::pages` manages the frontend displaying of requested data
pub fn routes(manager: PasteManager) -> Router {
    Router::new()
        .route("/:url", get(view_paste_by_url))
        .with_state(manager)
}
pub async fn root() -> &'static str {
    "A landing page will be displayed here, eventually with a code editor"
}
pub async fn not_found_handler() -> &'static str {
    "Error 404: the resource you requested could not be found"
}
#[derive(Template)]
#[template(path = "paste.html")]
struct PasteView {
    title:   String,
    paste:   PasteReturn,
}

// #[derive(Template)]
// #[template(path = "error.html")]
// struct ErrorView {
//     title:   String,
//     error:   PasteError,
// }

// TODO: handle askama errors gracefully
pub async fn view_paste_by_url(
    Path(url): Path<String>, 
    State(manager): State<PasteManager>
) -> impl IntoResponse {
    match manager.get_paste_by_url(url).await {
        Ok(p) => {
            let paste_render = PasteView {
                title: p.url.to_string(),
                paste: p
            };
            Html(paste_render.render().unwrap())
        },
        Err(e) => {
            let paste_render = PasteView {
                title: "error".to_string(),
                paste: PasteReturn {
                    url: "error".to_string(),
                    content: "error".to_string(),
                    date_published: 0,
                    date_edited: 0
                }
            };
            Html(paste_render.render().unwrap())
        }
    }

}