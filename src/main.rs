use pasties::{
    routing::api, 
    routing::pages,
    model::PasteManager
};
use axum::{
    Router, 
    routing::get,
};
#[tokio::main]
async fn main() {
    const PORT: u16 = 7878;

    let manager = PasteManager::init().await;

    let app = Router::new()
        .route("/", get(pages::root))
        .nest("/api", api::routes(manager.clone()));
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{PORT}")).await.unwrap();

    println!("Starting server at http://localhost:{PORT}!");
    axum::serve(listener, app).await.unwrap();
}