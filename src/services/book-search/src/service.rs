use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::{grpc::contracts::{book_search::{BookSearchResponse, Query}, common::{HealthCheckRequest, HealthCheckResponse}}, handlers::{book_search_handler, health_handler}};
use crate::grpc::contracts::book_search::book_search_grpc_server::BookSearchGrpc;
pub struct BookSearchService {
    pool: PgPool,
}

impl BookSearchService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl BookSearchGrpc for BookSearchService {
    async fn health_check(
    &self,
    request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = health_handler::health_check(request.into_inner())
        .await;

        Ok(Response::new(response))
    }
    async fn book_search(
        &self,
        _request: tonic::Request<Query>,
    ) -> Result<Response<BookSearchResponse>, Status> {

        let params = _request.into_inner();

        let response = book_search_handler::book_search(
            &self.pool,
            params,
        )
        .await
        .map_err(
        |e| Status::internal(e.to_string()))?;

        Ok(Response::new(response))
    }
}
