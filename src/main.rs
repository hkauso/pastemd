use axum::Router;
use pasties::{routing::api, DatabaseOpts, database::Database};
use std::env;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok(); // load .env

    let port: u16 = match env::var("PORT") {
        Ok(v) => v.parse::<u16>().unwrap(),
        Err(_) => 8080,
    };

    let manager = Database::new(DatabaseOpts {
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

    manager.init().await;

    let app = Router::new()
        .nest("/api", api::routes(manager.clone()))
        .fallback(api::not_found);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();

    println!("Starting server at http://localhost:{port}!");
    axum::serve(listener, app).await.unwrap();
}
