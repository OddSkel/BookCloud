use config::AppConfig;
use tonic::transport::Server;

use crate::{grpc::contracts::book_search::book_search_grpc_server::BookSearchGrpcServer, service::BookSearchService};

mod config;
mod grpc;
mod service;
mod handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address = config.grpc_address()?;
    
    println!("{} gRPC started on {}", config.service_name, address);

    Server::builder()
        .add_service(BookSearchGrpcServer::new(BookSearchService::new(
            config.book_catalog_grpc_url,
            config.search_page_size,
            config.search_max_pages,
        )))
        .serve(address)
        .await?;

    Ok(())
}
