use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::{grpc::contracts::{author_catalog::{self, AuthorsResponse, AuthorResponse, AuthorDeleteResponse}, common::{HealthCheckRequest, HealthCheckResponse}}, handlers::{author_handler, health_handler}};
use crate::grpc::contracts::author_catalog::author_catalog_grpc_server::AuthorCatalogGrpc;
pub struct AuthorCatalogService {
    pool: PgPool,
}

impl AuthorCatalogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl AuthorCatalogGrpc for AuthorCatalogService {
    async fn health_check(
    &self,
    request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = health_handler::health_check(request.into_inner())
        .await;

        Ok(Response::new(response))
    }
    async fn get_authors(
        &self,
        _request: tonic::Request<author_catalog::Empty>,
    ) -> Result<Response<AuthorsResponse>, Status> {

        let response = author_handler::get_authors(
            &self.pool)
            .await
            .map_err(
            |e| Status::internal(e.to_string()))?;

        Ok(Response::new(response))
    }

    async fn register_author(
        &self,
        _request: tonic::Request<author_catalog::AuthorParameters>,
    ) -> Result<Response<AuthorResponse>, Status> {

        let params = _request.into_inner();
        let response = author_handler::register_author(
            &self.pool, params)
            .await
            .map_err(
            |e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(response))
    }

    async fn edit_author(
        &self,
        _request: tonic::Request<author_catalog::AuthorEditParameters>
    )-> Result<Response<AuthorResponse>, Status> {

        let params = _request.into_inner();
        let response = author_handler::edit_author(
            &self.pool, params.id, params)
            .await
            .map_err(
                |e| Status::internal(e.to_string())
            )?;
        
        Ok(Response::new(response))
    }

    async fn delete_author(
        &self,
        _request: tonic::Request<author_catalog::AuthorId>
    ) -> Result<Response<AuthorDeleteResponse>, Status> {
        
        let params = _request.into_inner();
        let response = author_handler::delete_author(
            &self.pool, params)
            .await
            .map_err(
                |e| Status::internal(e.to_string())
            )?;

        Ok(Response::new(response))
    }

    async fn get_author(
        &self,
        _request: tonic::Request<author_catalog::AuthorId>
    ) -> Result<Response<AuthorResponse>, Status> {

        let params = _request.into_inner();
        let response = author_handler::get_author(&self.pool, params)
            .await
            .map_err(
                |e| Status::internal(e.to_string())
            )?;
        
        Ok(Response::new(response))
    }
}
