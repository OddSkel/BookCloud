use config::AppConfig;
use grpc::contracts::author_catalog::author_catalog_grpc_server::AuthorCatalogGrpcServer;
use service::AuthorCatalogService;
use tonic::transport::Server;

mod config;
mod grpc;
mod service;
mod handlers;
mod models;
mod db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address_str = config.grpc_address_string();
    let address = address_str.parse().expect("Failed to parse address");
    
    println!("DEBUG: Binding to address string: {}", address_str);
    println!("DEBUG: Parsed SocketAddr: {:?}", address);

    println!("{} gRPC started on {}", config.service_name, address_str);

    let pool = db::create_pool()
    .await
    .expect("Failed to connect to PostgreSQL");


    Server::builder()
        .add_service(AuthorCatalogGrpcServer::new(AuthorCatalogService::new(
            pool,
        )))
        .serve(address)
        .await
        .expect("serve() returned error");

    Ok(())
}
