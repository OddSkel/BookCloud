use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::{
    grpc::contracts::{
        book_recommendation::{BookRecommendationResponse, Query},
        common::{HealthCheckRequest, HealthCheckResponse}
    },
    handlers::{book_recommendation_handler, health_handler}
};
use crate::grpc::contracts::book_recommendation::book_recommendation_grpc_server::BookRecommendationGrpc;

pub struct BookRecommendationService {
    pool: PgPool,
}

impl BookRecommendationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl BookRecommendationGrpc for BookRecommendationService {
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = health_handler::health_check(request.into_inner()).await;

        Ok(Response::new(response))
    }
    
    async fn book_recommendation(
        &self,
        _request: Request<Query>,
    ) -> Result<Response<BookRecommendationResponse>, Status> {
        
        let params = _request.into_inner();

        let response = book_recommendation_handler::book_recommendation(
            &self.pool,
            params,
        )
        .await
        .map_err(Status::internal)?;

        Ok(Response::new(response))
    }
}