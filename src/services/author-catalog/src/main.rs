use config::AppConfig;
use grpc::contracts::author_catalog::author_catalog_grpc_server::AuthorCatalogGrpcServer;
use service::AuthorCatalogService;
use tonic::transport::Server;
use redis::Client;

mod config;
mod db;
mod grpc;
mod handlers;
mod metrics;
mod models;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address_str = config.grpc_address_string();
    let address = address_str.parse().expect("Failed to parse address");

    println!("DEBUG: Binding to address string: {}", address_str);
    println!("DEBUG: Parsed SocketAddr: {:?}", address);

    println!("{} gRPC started on {}", config.service_name, address_str);

    tokio::spawn(async {
        metrics::start_metrics_server("author-catalog").await;
    });

    let redis_client = Client::open(config.redis_url.as_str())
        .expect("Failed to create Redis client");
    let redis_conn = redis_client
        .get_connection_manager()
        .await
        .expect("Failed to connect to Redis");

    Server::builder()
        .add_service(AuthorCatalogGrpcServer::new(AuthorCatalogService::new(
            pool,
            redis_conn,
        )))
        .serve(address)
        .await
        .expect("serve() returned error");

    Ok(())
}
