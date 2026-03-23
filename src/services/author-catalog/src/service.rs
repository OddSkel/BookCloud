use tonic::{Request, Response, Status};

use crate::grpc::contracts::{
    author_catalog::author_catalog_grpc_server::AuthorCatalogGrpc,
    common::{HealthCheckRequest, HealthCheckResponse},
};

pub struct AuthorCatalogService {
    service_name: String,
}

impl AuthorCatalogService {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

#[tonic::async_trait]
impl AuthorCatalogGrpc for AuthorCatalogService {
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
