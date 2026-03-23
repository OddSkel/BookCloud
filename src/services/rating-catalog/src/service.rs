use tonic::{Request, Response, Status};

use crate::grpc::contracts::{
    common::{HealthCheckRequest, HealthCheckResponse},
    rating_catalog::rating_catalog_grpc_server::RatingCatalogGrpc,
};

pub struct RatingCatalogService {
    service_name: String,
}

impl RatingCatalogService {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

#[tonic::async_trait]
impl RatingCatalogGrpc for RatingCatalogService {
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
