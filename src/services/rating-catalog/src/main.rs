use config::AppConfig;
use grpc::contracts::rating_catalog::rating_catalog_grpc_server::RatingCatalogGrpcServer;
use service::RatingCatalogService;
use tonic::transport::Server;

mod config;
mod grpc;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address = config.grpc_address()?;

    println!("{} gRPC started on {}", config.service_name, address);

    Server::builder()
        .add_service(RatingCatalogGrpcServer::new(RatingCatalogService::new(
            config.service_name.clone(),
        )))
        .serve(address)
        .await?;

    Ok(())
}
