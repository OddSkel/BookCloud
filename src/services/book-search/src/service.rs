use tonic::{Request, Response, Status};

use crate::{grpc::contracts::{book_search::{BookSearchResponse, Query}, common::{HealthCheckRequest, HealthCheckResponse}}, handlers::{book_search_handler, health_handler}};
use crate::grpc::contracts::book_search::book_search_grpc_server::BookSearchGrpc;
pub struct BookSearchService {
    book_catalog_grpc_url: String,
    author_catalog_grpc_url: String,
    search_page_size: i32,
    search_max_pages: i32,
}

impl BookSearchService {
    pub fn new(book_catalog_grpc_url: String, author_catalog_grpc_url: String, search_page_size: i32, search_max_pages: i32) -> Self {
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
            &self.book_catalog_grpc_url,
            &self.author_catalog_grpc_url,
            self.search_page_size,
            self.search_max_pages,
            params,
        )
        .await
        .map_err(
        Status::internal)?;

        Ok(Response::new(response))
    }
}
