//! `routing::pages` responds to requests that should return rendered HTML to the client
use askama_axum::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Html},
    routing::{get, get_service},
    Router,
};
use tower_http::services::ServeDir;
use crate::model::Paste;
use crate::database::Database;

// `routing::pages` manages the frontend displaying of requested data
pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/:url", get(view_paste_by_url))
        .nest_service("/assets", get_service(ServeDir::new("./assets")))
        .with_state(database)
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
    title: String,
    paste: Paste,
}

//TODO: make an error page; handle askama errors gracefully instead of unwrapping

// #[derive(Template)]
// #[template(path = "error.html")]
// struct ErrorView {
//     title:   String,
//     error:   PasteError,
// }

pub async fn view_paste_by_url(
    Path(url): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    match database.get_paste_by_url(url).await {
        Ok(p) => {
            let paste_render = PasteView {
                title: p.url.to_string(),
                paste: p,
            };
            Html(paste_render.render().unwrap())
        }
        Err(_) => {
            let paste_render = PasteView {
                title: "error".to_string(),
                paste: Paste {
                    id: "error".to_string(),
                    url: "error".to_string(),
                    content: "error".to_string(),
                    password: "error".to_string(),
                    date_published: 0,
                    date_edited: 0,
                },
            };
            Html(paste_render.render().unwrap())
        }
    }
}
