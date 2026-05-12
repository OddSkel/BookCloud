use config::AppConfig;
use grpc::contracts::rating_catalog::rating_catalog_grpc_server::RatingCatalogGrpcServer;
use service::RatingCatalogService;
use tonic::transport::Server;
use redis::Client;

const RATING_GRPC_MESSAGE_SIZE_LIMIT: usize = 128 * 1024 * 1024;

mod config;
mod db;
mod grpc;
mod handlers;
mod models;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address = config.grpc_address()?;

    println!("{} gRPC started on {}", config.service_name, address);

    let pool = db::create_pool()
        .await
        .expect("Failed to connect to PostgreSQL");

    let redis_client = Client::open(config.redis_url.as_str())
        .expect("Failed to create Redis client");
    let redis_conn = redis_client
        .get_connection_manager()
        .await
        .expect("Failed to connect to Redis");

    Server::builder()
        .add_service(
            RatingCatalogGrpcServer::new(RatingCatalogService::new(
                config.service_name.clone(),
                pool,
                redis_conn,
            ))
            .max_decoding_message_size(RATING_GRPC_MESSAGE_SIZE_LIMIT)
            .max_encoding_message_size(RATING_GRPC_MESSAGE_SIZE_LIMIT),
        )
        .serve(address)
        .await?;

    Ok(())
}
