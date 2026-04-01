use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::contracts::{
    author_catalog::author_catalog_grpc_server::AuthorCatalogGrpc,
    common::{HealthCheckRequest, HealthCheckResponse},
};
use crate::grpc::contracts::author_catalog::{
    GetAuthorsRequest,
    GetAuthorsResponse,
    AddAuthorRequest,
    AddAuthorResponse,
    UpdateAuthorRequest,
    UpdateAuthorResponse,
    DeleteAuthorRequest,
    AuthorDeleteResponse,
    GetAuthorRequest,
    GetAuthorResponse,
    Author as ProtoAuthor
};
use crate::handlers::{author_handler, health_handler};


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
        _request: tonic::Request<GetAuthorsRequest>,
    ) -> Result<Response<GetAuthorsResponse>, Status> {

        let params = _request.into_inner();

        let response = author_handler::get_authors(
            &self.pool,
            &params)
            .await
            .map_err(
            |e| Status::internal(e.to_string()))?;

        Ok(Response::new(response))
    }

    async fn add_author(
        &self,
        _request: tonic::Request<AddAuthorRequest>,
    ) -> Result<Response<AddAuthorResponse>, Status> {

        let params = _request.into_inner();
        let proto_author = ProtoAuthor {
            id: params.id,
            name: params.name,
        };
        let response = author_handler::register_author(
            &self.pool, proto_author)
            .await
            .map_err(
            |e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(response))
    }

    async fn update_author(
        &self,
        _request: tonic::Request<UpdateAuthorRequest>
    )-> Result<Response<UpdateAuthorResponse>, Status> {

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
        _request: tonic::Request<DeleteAuthorRequest>
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
        _request: tonic::Request<GetAuthorRequest>
    ) -> Result<Response<GetAuthorResponse>, Status> {

        let params = _request.into_inner();
        let response = author_handler::get_author(&self.pool, params)
            .await
            .map_err(
                |e| Status::internal(e.to_string())
            )?;
        
        Ok(Response::new(response))
    }
}
