use config::AppConfig;
use grpc::contracts::author_catalog::author_catalog_grpc_server::AuthorCatalogGrpcServer;
use service::AuthorCatalogService;
use tonic::transport::Server;
use sqlx::postgres::PgPoolOptions;

mod config;
mod grpc;
mod service;
mod handlers;
mod models;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address = config.grpc_address()?;

    let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&config.db_url)
    .await
    .expect("Failed to connect to DB");
    
    println!("{} gRPC started on {}", config.service_name, address);

    Server::builder()
        .add_service(AuthorCatalogGrpcServer::new(AuthorCatalogService::new(
            pool,
        )))
        .serve(address)
        .await?;

    Ok(())
}
