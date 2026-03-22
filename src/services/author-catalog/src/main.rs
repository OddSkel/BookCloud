use actix_web::{App, HttpServer};
use routes::init_routes;
use std::env;

mod routes;
mod handlers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let service_name = env::var("SERVICE_NAME").unwrap_or_else(|_| "author-catalog".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("API_PORT").unwrap_or_else(|_| "8080".to_string());
    let address = format!("{}:{}", host, port);

    println!("{} started on {}", service_name, address);

    HttpServer::new(|| {
        App::new().configure(init_routes)
    })
    .bind(address)?
    .run()
    .await
}
