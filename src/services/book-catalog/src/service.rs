use tonic::{Request, Response, Status};

use crate::grpc::contracts::{
    book_catalog::book_catalog_grpc_server::BookCatalogGrpc,
    common::{HealthCheckRequest, HealthCheckResponse},
};

pub struct BookCatalogService {
    service_name: String,
}

impl BookCatalogService {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

#[tonic::async_trait]
impl BookCatalogGrpc for BookCatalogService {
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            service: self.service_name.clone(),
            status: "ok".to_string(),
        }))
    }
}
