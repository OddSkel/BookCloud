use actix_web::{App, HttpServer};
use dotenv::dotenv;
use sqlx::PgPool;
use tonic::transport::Server;

mod config;
mod db;
mod grpc;
mod handlers;
mod models;
mod routes;
mod service;
mod utils;

use config::AppConfig;
use db::AuthorAnalyticsDb;
use grpc::contracts::author_analytics::author_analytics_grpc_server::AuthorAnalyticsGrpcServer;
use routes::init_routes;
use service::AuthorAnalyticsService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config    = AppConfig::from_env();
    let grpc_addr = config.grpc_address().parse()?;
    let http_addr = config.http_address();

    println!("[{}] gRPC on {}", config.service_name, grpc_addr);
    println!("[{}] HTTP on {}", config.service_name, http_addr);

    let pool = PgPool::connect(&config.database_url).await
        .expect("Failed to connect to PostgreSQL");
    let db  = AuthorAnalyticsDb::new(pool);
    let svc = AuthorAnalyticsService::new(db);

    let http_addr_clone = http_addr.clone();
    tokio::spawn(async move {
        HttpServer::new(|| App::new().configure(init_routes))
            .bind(http_addr_clone).expect("Failed to bind HTTP")
            .run().await.expect("HTTP server failed");
    });

    Server::builder()
        .add_service(AuthorAnalyticsGrpcServer::new(svc))
        .serve(grpc_addr)
        .await?;

    Ok(())
}