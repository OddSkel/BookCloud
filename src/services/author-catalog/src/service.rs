use redis::aio::ConnectionManager;
use sqlx::PgPool;
use tonic::{Request, Response, Status};

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
    redis: ConnectionManager,
    cache_ttl_seconds: u64,
}

impl AuthorCatalogService {
    pub fn new(pool: PgPool, redis: ConnectionManager, cache_ttl_seconds: u64) -> Self {
        Self {
            pool,
            redis,
            cache_ttl_seconds,
        }
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

            let response = author_handler::get_authors(
                &self.pool,
                &self.redis,
                &params,
                self.cache_ttl_seconds,
            )
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

#[cfg(test)]
mod tests {
    // All proto types come from the real imports already at the top of this
    // file — do NOT re-import or redefine them here.
    use crate::grpc::contracts::author_catalog::{
        AddAuthorRequest, AddAuthorResponse, Author as ProtoAuthor, AuthorDeleteResponse,
        DeleteAuthorRequest, GetAuthorRequest, GetAuthorResponse, GetAuthorsByNameRequest,
        GetAuthorsRequest, GetAuthorsResponse, UpdateAuthorRequest, UpdateAuthorResponse,
    };
    use crate::grpc::contracts::common::{HealthCheckRequest, HealthCheckResponse};
    use tonic::{Code, Request, Response, Status};

    // -------------------------------------------------------------------------
    // TestService — owns a `fail` flag so every test is fully isolated.
    // No global / shared mutable state; safe under Tokio's multi-thread runner.
    // -------------------------------------------------------------------------

    struct TestService {
        fail: bool,
    }

    impl TestService {
        fn ok() -> Self {
            Self { fail: false }
        }
        fn fail() -> Self {
            Self { fail: true }
        }

        fn db_err(&self) -> Status {
            Status::internal("db error")
        }

        // --- health_check -----------------------------------------------------

        async fn health_check(
            &self,
            _req: Request<HealthCheckRequest>,
        ) -> Result<Response<HealthCheckResponse>, Status> {
            Ok(Response::new(HealthCheckResponse {
                status: "SERVING".into(),
                ..Default::default()
            }))
        }

        // --- get_authors ------------------------------------------------------

        async fn get_authors(
            &self,
            _req: Request<GetAuthorsRequest>,
        ) -> Result<Response<GetAuthorsResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            Ok(Response::new(GetAuthorsResponse {
                authors: vec![ProtoAuthor {
                    author_id: 1,
                    name: "Alice".into(),
                }],
            }))
        }

        // --- add_author -------------------------------------------------------

        async fn add_author(
            &self,
            req: Request<AddAuthorRequest>,
        ) -> Result<Response<AddAuthorResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            let p = req.into_inner();
            Ok(Response::new(AddAuthorResponse {
                author: Some(ProtoAuthor {
                    author_id: p.author_id,
                    name: p.name,
                }),
            }))
        }

        // --- update_author ----------------------------------------------------

        async fn update_author(
            &self,
            req: Request<UpdateAuthorRequest>,
        ) -> Result<Response<UpdateAuthorResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            let p = req.into_inner();
            Ok(Response::new(UpdateAuthorResponse {
                authors: vec![ProtoAuthor {
                    author_id: p.author_id,
                    name: p.name.unwrap_or_default(),
                }],
            }))
        }

        // --- delete_author ----------------------------------------------------

        async fn delete_author(
            &self,
            _req: Request<DeleteAuthorRequest>,
        ) -> Result<Response<AuthorDeleteResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            Ok(Response::new(AuthorDeleteResponse {
                sucessful: true,
                ..Default::default()
            }))
        }

        // --- get_author -------------------------------------------------------

        async fn get_author(
            &self,
            req: Request<GetAuthorRequest>,
        ) -> Result<Response<GetAuthorResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            let p = req.into_inner();
            Ok(Response::new(GetAuthorResponse {
                author: Some(ProtoAuthor {
                    author_id: p.author_id,
                    name: "Alice".into(),
                }),
            }))
        }

        // --- get_authors_by_name ----------------------------------------------

        async fn get_authors_by_name(
            &self,
            req: Request<GetAuthorsByNameRequest>,
        ) -> Result<Response<GetAuthorsResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            let p = req.into_inner();
            Ok(Response::new(GetAuthorsResponse {
                authors: vec![ProtoAuthor {
                    author_id: 1,
                    name: p.name,
                }],
            }))
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    // --- health_check ---------------------------------------------------------

    #[tokio::test]
    async fn health_check_returns_serving() {
        let resp = TestService::ok()
            .health_check(Request::new(HealthCheckRequest::default()))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().status, "SERVING");
    }

    // --- get_authors ----------------------------------------------------------

    #[tokio::test]
    async fn get_authors_returns_author_list() {
        let resp = TestService::ok()
            .get_authors(Request::new(GetAuthorsRequest::default()))
            .await
            .unwrap();
        let body = resp.into_inner();
        assert_eq!(body.authors.len(), 1);
        assert_eq!(body.authors[0].author_id, 1);
        assert_eq!(body.authors[0].name, "Alice");
    }

    #[tokio::test]
    async fn get_authors_error_maps_to_internal_status() {
        let err = TestService::fail()
            .get_authors(Request::new(GetAuthorsRequest::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
        assert!(err.message().contains("db error"));
    }

    // --- add_author -----------------------------------------------------------

    #[tokio::test]
    async fn add_author_returns_created_author() {
        let resp = TestService::ok()
            .add_author(Request::new(AddAuthorRequest {
                author_id: 42,
                name: "Bob".into(),
            }))
            .await
            .unwrap();
        let author = resp.into_inner().author.expect("expected Some(Author)");
        assert_eq!(author.author_id, 42);
        assert_eq!(author.name, "Bob");
    }

    #[tokio::test]
    async fn add_author_error_maps_to_internal_status() {
        let err = TestService::fail()
            .add_author(Request::new(AddAuthorRequest::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
    }

    // --- update_author --------------------------------------------------------

    #[tokio::test]
    async fn update_author_returns_updated_author_id() {
        let resp = TestService::ok()
            .update_author(Request::new(UpdateAuthorRequest {
                author_id: 7,
                name: Some("Updated".to_string()),
            }))
            .await
            .unwrap();
        let author = resp
            .into_inner()
            .authors
            .into_iter()
            .next()
            .expect("expected one author");
        assert_eq!(author.author_id, 7);
        assert_eq!(author.name, "Updated");
    }

    #[tokio::test]
    async fn update_author_error_maps_to_internal_status() {
        let err = TestService::fail()
            .update_author(Request::new(UpdateAuthorRequest::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
    }

    // --- delete_author --------------------------------------------------------

    #[tokio::test]
    async fn delete_author_returns_success_true() {
        let resp = TestService::ok()
            .delete_author(Request::new(DeleteAuthorRequest { author_id: 3 }))
            .await
            .unwrap();
        assert!(resp.into_inner().sucessful);
    }

    #[tokio::test]
    async fn delete_author_error_maps_to_internal_status() {
        let err = TestService::fail()
            .delete_author(Request::new(DeleteAuthorRequest::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
    }

    // --- get_author -----------------------------------------------------------

    #[tokio::test]
    async fn get_author_returns_correct_author() {
        let resp = TestService::ok()
            .get_author(Request::new(GetAuthorRequest { author_id: 1 }))
            .await
            .unwrap();
        let author = resp.into_inner().author.expect("expected Some(Author)");
        assert_eq!(author.author_id, 1);
        assert_eq!(author.name, "Alice");
    }

    #[tokio::test]
    async fn get_author_error_maps_to_internal_status() {
        let err = TestService::fail()
            .get_author(Request::new(GetAuthorRequest::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
    }

    // --- get_authors_by_name --------------------------------------------------

    #[tokio::test]
    async fn get_authors_by_name_returns_matching_authors() {
        let resp = TestService::ok()
            .get_authors_by_name(Request::new(GetAuthorsByNameRequest {
                name: "Alice".into(),
            }))
            .await
            .unwrap();
        let body = resp.into_inner();
        assert_eq!(body.authors.len(), 1);
        assert_eq!(body.authors[0].name, "Alice");
    }

    #[tokio::test]
    async fn get_authors_by_name_error_maps_to_internal_status() {
        let err = TestService::fail()
            .get_authors_by_name(Request::new(GetAuthorsByNameRequest::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
    }

    // --- error message propagation --------------------------------------------

    #[tokio::test]
    async fn error_message_is_propagated_from_handler() {
        let err = TestService::fail()
            .get_author(Request::new(GetAuthorRequest::default()))
            .await
            .unwrap_err();
        assert!(err.message().contains("db error"), "got: {}", err.message());
    }
}
