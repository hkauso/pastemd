use axum::{routing::get, Router};
use pasties::{model::PasteManager, routing::api, routing::pages, DatabaseOpts};
use std::env;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    const PORT: u16 = 7878;

    let manager = PasteManager::init(DatabaseOpts {
        // dorsal expects "_type" and "host" to be Option but "env::var" gives Result...
        // we just need to convert the result to an option
        _type: match env::var("DB_TYPE") {
            Ok(v) => Option::Some(v),
            Err(_) => Option::None,
        },
        host: match env::var("DB_HOST") {
            Ok(v) => Option::Some(v),
            Err(_) => Option::None,
        },
        user: env::var("DB_USER").unwrap_or(String::new()),
        pass: env::var("DB_PASS").unwrap_or(String::new()),
        name: env::var("DB_NAME").unwrap_or(String::new()),
    })
    .await;

    let app = Router::new()
        .route("/", get(pages::root))
        .merge(pages::routes(manager.clone()))
        .nest("/api", api::routes(manager.clone()))
        .fallback(pages::not_found_handler);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{PORT}"))
        .await
        .unwrap();

    println!("Starting server at http://localhost:{PORT}!");
    axum::serve(listener, app).await.unwrap();
}
