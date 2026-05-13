use sqlx::PgPool;
use tonic::{Request, Response, Status};
use redis::aio::ConnectionManager;

use crate::grpc::contracts::author_catalog::{
    AddAuthorRequest, AddAuthorResponse, Author as ProtoAuthor, AuthorDeleteResponse,
    DeleteAuthorRequest, GetAuthorRequest, GetAuthorResponse, GetAuthorsByNameRequest,
    GetAuthorsRequest, GetAuthorsResponse, UpdateAuthorRequest, UpdateAuthorResponse,
};
use crate::grpc::contracts::{
    author_catalog::author_catalog_grpc_server::AuthorCatalogGrpc,
    common::{HealthCheckRequest, HealthCheckResponse},
};
use crate::handlers::{author_handler, health_handler};
use crate::metrics;

pub struct AuthorCatalogService {
    pool: PgPool,
    redis: ConnectionManager
}

impl AuthorCatalogService {
    pub fn new(pool: PgPool, redis: ConnectionManager) -> Self {
        Self { pool, redis }
    }
}

#[tonic::async_trait]
impl AuthorCatalogGrpc for AuthorCatalogService {
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = health_handler::health_check(request.into_inner()).await;

        Ok(Response::new(response))
    }
    
    async fn get_authors(
        &self,
        request: tonic::Request<GetAuthorsRequest>,
    ) -> Result<Response<GetAuthorsResponse>, Status> {
        let operation = "list_authors";
        let timer = metrics::start_timer("author-catalog", operation);

        let result = async {
            let params = request.into_inner();

            let response = author_handler::get_authors(&self.pool, &self.redis, &params)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("author-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("author-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn add_author(
        &self,
        _request: tonic::Request<AddAuthorRequest>,
    ) -> Result<Response<AddAuthorResponse>, Status> {
        let operation = "create_author";
        let timer = metrics::start_timer("author-catalog", operation);

        let result = async {
            let params = _request.into_inner();
            let proto_author = ProtoAuthor {
                author_id: params.author_id,
                name: params.name,
            };
            let response = author_handler::register_author(&self.pool, proto_author)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("author-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("author-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn update_author(
        &self,
        _request: tonic::Request<UpdateAuthorRequest>,
    ) -> Result<Response<UpdateAuthorResponse>, Status> {
        let operation = "update_author";
        let timer = metrics::start_timer("author-catalog", operation);

        let result = async {
            let params = _request.into_inner();
            let response = author_handler::edit_author(&self.pool, params.author_id, params)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("author-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("author-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn delete_author(
        &self,
        _request: tonic::Request<DeleteAuthorRequest>,
    ) -> Result<Response<AuthorDeleteResponse>, Status> {
        let operation = "delete_author";
        let timer = metrics::start_timer("author-catalog", operation);

        let result = async {
            let params = _request.into_inner();
            let response = author_handler::delete_author(&self.pool, params)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("author-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("author-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn get_author(
        &self,
        _request: tonic::Request<GetAuthorRequest>,
    ) -> Result<Response<GetAuthorResponse>, Status> {
        let operation = "get_author";
        let timer = metrics::start_timer("author-catalog", operation);

        let result = async {
            let params = _request.into_inner();
            let response = author_handler::get_author(&self.pool, params)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("author-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("author-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn get_authors_by_name(
        &self,
        request: tonic::Request<GetAuthorsByNameRequest>,
    ) -> Result<Response<GetAuthorsResponse>, Status> {
        let operation = "search_authors";
        let timer = metrics::start_timer("author-catalog", operation);

        let result = async {
            let params = request.into_inner();

            let response = author_handler::get_authors_by_name(&self.pool, &params)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("author-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("author-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }
}
