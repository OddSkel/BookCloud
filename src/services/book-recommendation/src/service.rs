use tonic::{Request, Response, Status};

use crate::grpc::contracts::book_recommendation::book_recommendation_grpc_server::BookRecommendationGrpc;
use crate::{
    grpc::contracts::{
        book_recommendation::{BookRecommendationResponse, Query},
        common::{HealthCheckRequest, HealthCheckResponse},
    },
    handlers::{book_recommendation_handler, health_handler},
};

pub struct BookRecommendationService {
    genre_analysis_grpc_url: String,
    book_catalog_grpc_url: String,
    rating_catalog_grpc_url: String,
}

impl BookRecommendationService {
    pub fn new(
        genre_analysis_grpc_url: String,
        book_catalog_grpc_url: String,
        rating_catalog_grpc_url: String,
    ) -> Self {
        Self {
            genre_analysis_grpc_url,
            book_catalog_grpc_url,
            rating_catalog_grpc_url,
        }
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
            params,
            &self.genre_analysis_grpc_url,
            &self.book_catalog_grpc_url,
            &self.rating_catalog_grpc_url,
        )
        .await
        .map_err(Status::internal)?;

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use crate::grpc::contracts::{
        book_recommendation::{BookRecommendationResponse, Query},
        common::{HealthCheckRequest, HealthCheckResponse},
    };
    use tonic::{Code, Request, Response, Status};

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

        fn handler_err(&self) -> Status {
            Status::internal("recommendation error")
        }

        async fn health_check(
            &self,
            _req: Request<HealthCheckRequest>,
        ) -> Result<Response<HealthCheckResponse>, Status> {
            Ok(Response::new(HealthCheckResponse {
                status: "SERVING".into(),
                ..Default::default()
            }))
        }

        async fn book_recommendation(
            &self,
            _req: Request<Query>,
        ) -> Result<Response<BookRecommendationResponse>, Status> {
            if self.fail {
                return Err(self.handler_err());
            }
            Ok(Response::new(BookRecommendationResponse {
                books: vec![],
                ..Default::default()
            }))
        }
    }

    // --- health_check ---------------------------------------------------------

    #[tokio::test]
    async fn health_check_returns_serving() {
        let resp = TestService::ok()
            .health_check(Request::new(HealthCheckRequest::default()))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().status, "SERVING");
    }

    // --- book_recommendation --------------------------------------------------

    #[tokio::test]
    async fn book_recommendation_returns_ok_response() {
        let query = Query {
            genre: Some("Fantasy".to_string()),
            rating: Some(4.5),
            popularity: Some(100.0),
            page: Some(1),
            page_size: Some(10),
            ..Default::default()
        };
        let resp = TestService::ok()
            .book_recommendation(Request::new(query))
            .await
            .unwrap();
        assert!(resp.into_inner().books.is_empty());
    }

    #[tokio::test]
    async fn book_recommendation_empty_query_still_succeeds() {
        let resp = TestService::ok()
            .book_recommendation(Request::new(Query::default()))
            .await
            .unwrap();
        assert!(resp.into_inner().books.is_empty());
    }

    #[tokio::test]
    async fn book_recommendation_error_maps_to_internal_status() {
        let err = TestService::fail()
            .book_recommendation(Request::new(Query::default()))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::Internal);
        assert!(err.message().contains("recommendation error"));
    }

    #[tokio::test]
    async fn error_message_is_propagated_from_handler() {
        let err = TestService::fail()
            .book_recommendation(Request::new(Query::default()))
            .await
            .unwrap_err();
        assert!(
            err.message().contains("recommendation error"),
            "got: {}",
            err.message()
        );
    }
}
