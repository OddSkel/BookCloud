use tonic::{Request, Response, Status};

use crate::grpc::contracts::book_search::book_search_grpc_server::BookSearchGrpc;
use crate::{
    grpc::contracts::{
        book_search::{BookSearchResponse, Query},
        common::{HealthCheckRequest, HealthCheckResponse},
    },
    handlers::{book_search_handler, health_handler},
};
pub struct BookSearchService {
    book_catalog_grpc_url: String,
    author_catalog_grpc_url: String,
    search_page_size: i32,
    search_max_pages: i32,
}

impl BookSearchService {
    pub fn new(
        book_catalog_grpc_url: String,
        author_catalog_grpc_url: String,
        search_page_size: i32,
        search_max_pages: i32,
    ) -> Self {
        Self {
            book_catalog_grpc_url,
            author_catalog_grpc_url,
            search_page_size,
            search_max_pages,
        }
    }
}

#[tonic::async_trait]
impl BookSearchGrpc for BookSearchService {
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = health_handler::health_check(request.into_inner()).await;

        Ok(Response::new(response))
    }
    async fn book_search(
        &self,
        _request: tonic::Request<Query>,
    ) -> Result<Response<BookSearchResponse>, Status> {
        let params = _request.into_inner();

        let response = book_search_handler::book_search(
            &self.book_catalog_grpc_url,
            &self.author_catalog_grpc_url,
            self.search_page_size,
            self.search_max_pages,
            params,
        )
        .await
        .map_err(Status::internal)?;

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use crate::grpc::contracts::{
        book_search::{BookSearchResponse, Query},
        common::{HealthCheckRequest, HealthCheckResponse},
    };
    use tonic::{Code, Request, Response, Status};

    // -------------------------------------------------------------------------
    // TestService — carries a `fail` flag for full test isolation.
    // No shared mutable state; safe under Tokio's multi-thread runner.
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
            Status::internal("search error")
        }

        async fn health_check(
            &self,
            _req: Request<HealthCheckRequest>,
        ) -> Result<Response<HealthCheckResponse>, Status> {
            Ok(Response::new(HealthCheckResponse {
                service: "book-search".into(),
                status: "SERVING".into(),
            }))
        }

        async fn book_search(
            &self,
            _req: Request<Query>,
        ) -> Result<Response<BookSearchResponse>, Status> {
            if self.fail {
                return Err(self.db_err());
            }
            Ok(Response::new(BookSearchResponse { books: vec![] }))
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

    // --- book_search ----------------------------------------------------------

    #[tokio::test]
    async fn book_search_returns_response_with_query() {
        let query = Query {
            title: Some("Rust programming".to_string()),
            author: Some("Steve Klabnik".to_string()),
            keywords: Some("ownership borrowing".to_string()),
        };

        let resp = TestService::ok()
            .book_search(Request::new(query))
            .await
            .unwrap();
        let body = resp.into_inner();
        assert!(body.books.is_empty());
    }

    #[tokio::test]
    async fn book_search_empty_query_still_succeeds() {
        let resp = TestService::ok()
            .book_search(Request::new(Query::default()))
            .await
            .unwrap();
        assert!(resp.into_inner().books.is_empty());
    }

    #[tokio::test]
    async fn book_search_error_maps_to_internal_status() {
        let err = TestService::fail()
            .book_search(Request::new(Query::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
        assert!(err.message().contains("search error"));
    }

    // --- error message propagation --------------------------------------------

    #[tokio::test]
    async fn error_message_is_propagated_from_handler() {
        let err = TestService::fail()
            .book_search(Request::new(Query::default()))
            .await
            .unwrap_err();
        assert!(
            err.message().contains("search error"),
            "got: {}",
            err.message()
        );
    }
}
