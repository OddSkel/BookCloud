use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::grpc::contracts::rating_catalog::{
    AddRatingRequest, AddRatingResponse, BookRatingInfo, DeleteRatingRequest, DeleteRatingResponse,
    GetBooksByPopularityRequest, GetBooksByPopularityResponse, GetBooksByRatingRequest,
    GetBooksByRatingResponse, GetRatingsRequest, GetRatingsResponse, UpdateRatingRequest,
    UpdateRatingResponse,
};
use crate::handlers::rating_handler;
use crate::metrics;
use crate::models::rating::Rating;

use crate::grpc::contracts::{
    rating_catalog::rating_catalog_grpc_server::RatingCatalogGrpc,
    rating_catalog::{GetRatingRequest, GetRatingResponse, Rating as GrpcRating},
};

pub struct RatingCatalogService {
    service_name: String,
    backend: Arc<dyn RatingCatalogBackend>,
}

impl RatingCatalogService {
    pub fn new(
        service_name: String,
        pool: PgPool,
        redis: ConnectionManager,
        cache_ttl_seconds: u64,
    ) -> Self {
        Self {
            service_name,
            backend: Arc::new(PostgresRatingCatalogBackend {
                pool,
                redis,
                cache_ttl_seconds,
            }),
        }
    }

    #[cfg(test)]
    fn with_backend(service_name: String, backend: Arc<dyn RatingCatalogBackend>) -> Self {
        Self {
            service_name,
            backend,
        }
    }
}

#[tonic::async_trait]
trait RatingCatalogBackend: Send + Sync {
    async fn get_rating(&self, book_isbn: i64) -> Result<Option<Rating>, Status>;
    async fn get_ratings(
        &self,
        page_number: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Vec<Rating>, Status>;
    async fn add_rating(
        &self,
        book_isbn: i64,
        num_ratings: i64,
        star_rating: f64,
    ) -> Result<Rating, Status>;
    async fn update_rating(
        &self,
        book_isbn: i64,
        num_ratings: i64,
        star_rating: f64,
    ) -> Result<Option<Rating>, Status>;
    async fn delete_rating(&self, book_isbn: i64) -> Result<bool, Status>;
    async fn get_books_by_rating(
        &self,
        min_rating: f64,
        page_num: i32,
        page_size: i32,
    ) -> Result<Vec<Rating>, Status>;
    async fn get_books_by_popularity(
        &self,
        min_num_ratings: i32,
        page_num: i32,
        page_size: i32,
    ) -> Result<Vec<Rating>, Status>;
}

struct PostgresRatingCatalogBackend {
    pool: PgPool,
    redis: ConnectionManager,
    cache_ttl_seconds: u64,
}

#[tonic::async_trait]
impl RatingCatalogBackend for PostgresRatingCatalogBackend {
    async fn get_rating(&self, book_isbn: i64) -> Result<Option<Rating>, Status> {
        rating_handler::get_rating(&self.pool, book_isbn)
            .await
            .map_err(|err| Status::internal(err.to_string()))
    }

    async fn get_ratings(
        &self,
        page_number: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Vec<Rating>, Status> {
        rating_handler::get_ratings(
            &self.pool,
            &self.redis,
            page_number,
            page_size,
            self.cache_ttl_seconds,
        )
        .await
        .map_err(|err| Status::internal(err.to_string()))
    }

    async fn add_rating(
        &self,
        book_isbn: i64,
        num_ratings: i64,
        star_rating: f64,
    ) -> Result<Rating, Status> {
        rating_handler::add_rating(&self.pool, book_isbn, num_ratings, star_rating)
            .await
            .map_err(|err| Status::internal(err.to_string()))
    }

    async fn update_rating(
        &self,
        book_isbn: i64,
        num_ratings: i64,
        star_rating: f64,
    ) -> Result<Option<Rating>, Status> {
        rating_handler::update_rating(&self.pool, book_isbn, num_ratings, star_rating)
            .await
            .map_err(|err| Status::internal(err.to_string()))
    }

    async fn delete_rating(&self, book_isbn: i64) -> Result<bool, Status> {
        rating_handler::delete_rating(&self.pool, book_isbn)
            .await
            .map_err(|err| Status::internal(err.to_string()))
    }

    async fn get_books_by_rating(
        &self,
        min_rating: f64,
        page_num: i32,
        page_size: i32,
    ) -> Result<Vec<Rating>, Status> {
        rating_handler::get_books_by_rating(&self.pool, min_rating, page_num, page_size)
            .await
            .map_err(|err| Status::internal(err.to_string()))
    }

    async fn get_books_by_popularity(
        &self,
        min_num_ratings: i32,
        page_num: i32,
        page_size: i32,
    ) -> Result<Vec<Rating>, Status> {
        rating_handler::get_books_by_popularity(&self.pool, min_num_ratings, page_num, page_size)
            .await
            .map_err(|err| Status::internal(err.to_string()))
    }
}

fn parse_book_isbn(book_isbn: &str) -> Result<i64, Status> {
    book_isbn
        .parse()
        .map_err(|_| Status::invalid_argument("Invalid book_isbn"))
}

fn map_rating(rating: Rating) -> GrpcRating {
    GrpcRating {
        book_isbn: rating.book_isbn.to_string(),
        star_rating: rating.star_rating,
        num_ratings: rating.num_ratings,
    }
}

fn normalize_ranked_books_pagination(page_num: i32, page_size: i32) -> (i32, i32) {
    (page_num.max(1), page_size.clamp(1, 1000))
}

#[tonic::async_trait]
impl RatingCatalogGrpc for RatingCatalogService {
    async fn get_rating(
        &self,
        request: Request<GetRatingRequest>,
    ) -> Result<Response<GetRatingResponse>, Status> {
        let operation = "get_rating";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();

            let book_isbn = parse_book_isbn(&req.book_isbn)?;

            let rating = self
                .backend
                .get_rating(book_isbn)
                .await?
                .ok_or_else(|| Status::not_found("Rating not found"))?;

            let response = GetRatingResponse {
                rating: Some(map_rating(rating)),
            };

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }
    async fn get_ratings(
        &self,
        request: Request<GetRatingsRequest>,
    ) -> Result<Response<GetRatingsResponse>, Status> {
        let operation = "list_ratings";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();

            let ratings: Vec<Rating> = self
                .backend
                .get_ratings(req.page_number, req.page_size)
                .await?;

            let response = GetRatingsResponse {
                ratings: ratings.into_iter().map(map_rating).collect(),
            };

            Ok(Response::new(response))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn add_rating(
        &self,
        request: Request<AddRatingRequest>,
    ) -> Result<Response<AddRatingResponse>, Status> {
        let operation = "create_rating";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();
            let book_isbn = parse_book_isbn(&req.book_isbn)?;
            let payload = req
                .rating
                .ok_or_else(|| Status::invalid_argument("Missing rating payload"))?;

            let rating = self
                .backend
                .add_rating(book_isbn, payload.num_ratings, payload.star_rating)
                .await?;

            Ok(Response::new(AddRatingResponse {
                rating: Some(map_rating(rating)),
            }))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn update_rating(
        &self,
        request: Request<UpdateRatingRequest>,
    ) -> Result<Response<UpdateRatingResponse>, Status> {
        let operation = "update_rating";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();
            let book_isbn = parse_book_isbn(&req.book_isbn)?;

            let rating = self
                .backend
                .update_rating(book_isbn, req.num_ratings, req.star_rating)
                .await?
                .ok_or_else(|| Status::not_found("Rating not found"))?;

            Ok(Response::new(UpdateRatingResponse {
                ratings: vec![map_rating(rating)],
            }))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn delete_rating(
        &self,
        request: Request<DeleteRatingRequest>,
    ) -> Result<Response<DeleteRatingResponse>, Status> {
        let operation = "delete_rating";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();
            let book_isbn = parse_book_isbn(&req.book_isbn)?;

            let deleted = self.backend.delete_rating(book_isbn).await?;

            if !deleted {
                return Err(Status::not_found("Rating not found"));
            }

            Ok(Response::new(DeleteRatingResponse {}))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn get_books_by_rating(
        &self,
        request: Request<GetBooksByRatingRequest>,
    ) -> Result<Response<GetBooksByRatingResponse>, Status> {
        let operation = "get_book_rating";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();
            let (page_num, page_size) =
                normalize_ranked_books_pagination(req.page_num, req.page_size);

            let ratings = self
                .backend
                .get_books_by_rating(req.min_rating, page_num, page_size)
                .await?;

            let total_items = ratings.len() as i32;
            let total_pages = (total_items + page_size - 1) / page_size;

            Ok(Response::new(GetBooksByRatingResponse {
                items: ratings
                    .into_iter()
                    .map(|r| BookRatingInfo {
                        book_isbn: r.book_isbn.to_string(),
                        num_ratings: r.num_ratings,
                        star_rating: r.star_rating,
                    })
                    .collect(),
                page_num,
                page_size,
                total_items,
                total_pages,
            }))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }

    async fn get_books_by_popularity(
        &self,
        request: Request<GetBooksByPopularityRequest>,
    ) -> Result<Response<GetBooksByPopularityResponse>, Status> {
        let operation = "get_books_by_popularity";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();
            let (page_num, page_size) =
                normalize_ranked_books_pagination(req.page_num, req.page_size);

            let ratings = self
                .backend
                .get_books_by_popularity(req.min_num_ratings, page_num, page_size)
                .await?;

            let total_items = ratings.len() as i32;
            let total_pages = (total_items + page_size - 1) / page_size;

            Ok(Response::new(GetBooksByPopularityResponse {
                items: ratings
                    .into_iter()
                    .map(|r| BookRatingInfo {
                        book_isbn: r.book_isbn.to_string(),
                        num_ratings: r.num_ratings,
                        star_rating: r.star_rating,
                    })
                    .collect(),
                page_num,
                page_size,
                total_items,
                total_pages,
            }))
        }
        .await;

        match result {
            Ok(response) => {
                metrics::record_success("rating-catalog", operation);
                drop(timer);
                Ok(response)
            }
            Err(error) => {
                metrics::record_error("rating-catalog", operation);
                drop(timer);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::contracts::rating_catalog::{
        AddRatingRequest, DeleteRatingRequest, GetBooksByPopularityRequest, GetBooksByRatingRequest,
        GetRatingRequest, GetRatingsRequest, RatingAdd, UpdateRatingRequest,
    };
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRatingBackend {
        ratings: Mutex<HashMap<i64, Rating>>,
    }

    impl FakeRatingBackend {
        fn seeded() -> Arc<Self> {
            let backend = Arc::new(Self::default());
            {
                let mut ratings = backend.ratings.lock().unwrap();
                ratings.insert(
                    1,
                    Rating {
                        book_isbn: 1,
                        star_rating: 4.8,
                        num_ratings: 50,
                    },
                );
                ratings.insert(
                    2,
                    Rating {
                        book_isbn: 2,
                        star_rating: 3.1,
                        num_ratings: 500,
                    },
                );
            }
            backend
        }

        fn paginated(mut ratings: Vec<Rating>, page_num: i32, page_size: i32) -> Vec<Rating> {
            let start = ((page_num - 1) * page_size) as usize;
            let end = start + page_size as usize;
            if start >= ratings.len() {
                return Vec::new();
            }
            ratings.drain(start..ratings.len().min(end)).collect()
        }
    }

    #[tonic::async_trait]
    impl RatingCatalogBackend for FakeRatingBackend {
        async fn get_rating(&self, book_isbn: i64) -> Result<Option<Rating>, Status> {
            Ok(self.ratings.lock().unwrap().get(&book_isbn).cloned())
        }

        async fn get_ratings(
            &self,
            page_number: Option<i64>,
            page_size: Option<i64>,
        ) -> Result<Vec<Rating>, Status> {
            let mut ratings = self
                .ratings
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect::<Vec<_>>();
            ratings.sort_by_key(|rating| rating.book_isbn);

            let page_number = page_number.unwrap_or(1).max(1);
            let page_size = page_size.unwrap_or(10).max(1);
            let start = ((page_number - 1) * page_size) as usize;
            let end = start + page_size as usize;
            Ok(if start >= ratings.len() {
                Vec::new()
            } else {
                ratings.drain(start..ratings.len().min(end)).collect()
            })
        }

        async fn add_rating(
            &self,
            book_isbn: i64,
            num_ratings: i64,
            star_rating: f64,
        ) -> Result<Rating, Status> {
            let rating = Rating {
                book_isbn,
                star_rating,
                num_ratings,
            };
            self.ratings
                .lock()
                .unwrap()
                .insert(book_isbn, rating.clone());
            Ok(rating)
        }

        async fn update_rating(
            &self,
            book_isbn: i64,
            num_ratings: i64,
            star_rating: f64,
        ) -> Result<Option<Rating>, Status> {
            let mut ratings = self.ratings.lock().unwrap();
            if !ratings.contains_key(&book_isbn) {
                return Ok(None);
            }

            let rating = Rating {
                book_isbn,
                star_rating,
                num_ratings,
            };
            ratings.insert(book_isbn, rating.clone());
            Ok(Some(rating))
        }

        async fn delete_rating(&self, book_isbn: i64) -> Result<bool, Status> {
            Ok(self.ratings.lock().unwrap().remove(&book_isbn).is_some())
        }

        async fn get_books_by_rating(
            &self,
            min_rating: f64,
            page_num: i32,
            page_size: i32,
        ) -> Result<Vec<Rating>, Status> {
            let mut ratings = self
                .ratings
                .lock()
                .unwrap()
                .values()
                .filter(|rating| rating.star_rating >= min_rating)
                .cloned()
                .collect::<Vec<_>>();
            ratings.sort_by(|left, right| {
                right
                    .star_rating
                    .partial_cmp(&left.star_rating)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            Ok(Self::paginated(ratings, page_num, page_size))
        }

        async fn get_books_by_popularity(
            &self,
            min_num_ratings: i32,
            page_num: i32,
            page_size: i32,
        ) -> Result<Vec<Rating>, Status> {
            let mut ratings = self
                .ratings
                .lock()
                .unwrap()
                .values()
                .filter(|rating| rating.num_ratings >= min_num_ratings as i64)
                .cloned()
                .collect::<Vec<_>>();
            ratings.sort_by_key(|right| std::cmp::Reverse(right.num_ratings));
            Ok(Self::paginated(ratings, page_num, page_size))
        }
    }

    fn service() -> (RatingCatalogService, Arc<FakeRatingBackend>) {
        let backend = FakeRatingBackend::seeded();
        (
            RatingCatalogService::with_backend("rating-catalog".to_string(), backend.clone()),
            backend,
        )
    }

    #[tokio::test]
    async fn get_ratings_endpoint_returns_paginated_ratings() {
        let (service, _) = service();

        let response = service
            .get_ratings(Request::new(GetRatingsRequest {
                page_number: Some(1),
                page_size: Some(1),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.ratings.len(), 1);
        assert_eq!(response.ratings[0].book_isbn, "1");
    }

    #[tokio::test]
    async fn get_rating_endpoint_returns_one_rating() {
        let (service, _) = service();

        let response = service
            .get_rating(Request::new(GetRatingRequest {
                book_isbn: "2".to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        let rating = response.rating.unwrap();
        assert_eq!(rating.book_isbn, "2");
        assert_eq!(rating.num_ratings, 500);
    }

    #[tokio::test]
    async fn add_rating_endpoint_creates_rating() {
        let (service, backend) = service();

        let response = service
            .add_rating(Request::new(AddRatingRequest {
                book_isbn: "3".to_string(),
                rating: Some(RatingAdd {
                    num_ratings: 25,
                    star_rating: 4.2,
                }),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.rating.unwrap().book_isbn, "3");
        assert!(backend.ratings.lock().unwrap().contains_key(&3));
    }

    #[tokio::test]
    async fn update_rating_endpoint_updates_rating() {
        let (service, _) = service();

        let response = service
            .update_rating(Request::new(UpdateRatingRequest {
                book_isbn: "1".to_string(),
                num_ratings: 60,
                star_rating: 4.9,
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.ratings[0].book_isbn, "1");
        assert_eq!(response.ratings[0].num_ratings, 60);
        assert_eq!(response.ratings[0].star_rating, 4.9);
    }

    #[tokio::test]
    async fn delete_rating_endpoint_deletes_rating() {
        let (service, backend) = service();

        service
            .delete_rating(Request::new(DeleteRatingRequest {
                book_isbn: "1".to_string(),
            }))
            .await
            .unwrap();

        assert!(!backend.ratings.lock().unwrap().contains_key(&1));
    }

    #[tokio::test]
    async fn get_books_by_rating_endpoint_filters_and_sorts() {
        let (service, _) = service();

        let response = service
            .get_books_by_rating(Request::new(GetBooksByRatingRequest {
                min_rating: 3.0,
                page_num: 1,
                page_size: 10,
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].book_isbn, "1");
    }

    #[tokio::test]
    async fn get_books_by_popularity_endpoint_filters_and_sorts() {
        let (service, _) = service();

        let response = service
            .get_books_by_popularity(Request::new(GetBooksByPopularityRequest {
                min_num_ratings: 40,
                page_num: 1,
                page_size: 10,
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].book_isbn, "2");
    }

    #[tokio::test]
    async fn get_rating_not_found_returns_error() {
        let (service, _) = service();

        let result = service
            .get_rating(Request::new(GetRatingRequest {
                book_isbn: "999".to_string(),
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn get_rating_invalid_isbn_returns_invalid_argument() {
        let (service, _) = service();

        let result = service
            .get_rating(Request::new(GetRatingRequest {
                book_isbn: "not-a-number".to_string(),
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn add_rating_missing_payload_returns_invalid_argument() {
        let (service, _) = service();

        let result = service
            .add_rating(Request::new(AddRatingRequest {
                book_isbn: "1".to_string(),
                rating: None,
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn update_nonexistent_rating_returns_not_found() {
        let (service, _) = service();

        let result = service
            .update_rating(Request::new(UpdateRatingRequest {
                book_isbn: "999".to_string(),
                num_ratings: 10,
                star_rating: 3.0,
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn delete_nonexistent_rating_returns_not_found() {
        let (service, _) = service();

        let result = service
            .delete_rating(Request::new(DeleteRatingRequest {
                book_isbn: "999".to_string(),
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn get_ratings_empty_page_returns_empty() {
        let (service, _) = service();

        let response = service
            .get_ratings(Request::new(GetRatingsRequest {
                page_number: Some(999),
                page_size: Some(10),
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(response.ratings.is_empty());
    }

    #[tokio::test]
    async fn get_books_by_rating_no_matches_returns_empty() {
        let (service, _) = service();

        let response = service
            .get_books_by_rating(Request::new(GetBooksByRatingRequest {
                min_rating: 5.1,
                page_num: 1,
                page_size: 10,
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(response.items.is_empty());
    }

    #[tokio::test]
    async fn get_books_by_popularity_no_matches_returns_empty() {
        let (service, _) = service();

        let response = service
            .get_books_by_popularity(Request::new(GetBooksByPopularityRequest {
                min_num_ratings: 1000,
                page_num: 1,
                page_size: 10,
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(response.items.is_empty());
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn parse_book_isbn_valid() {
        let result = parse_book_isbn("12345");
        assert_eq!(result.unwrap(), 12345i64);
    }

    #[test]
    fn parse_book_isbn_invalid() {
        let result = parse_book_isbn("abc");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn parse_book_isbn_negative() {
        let result = parse_book_isbn("-1");
        assert_eq!(result.unwrap(), -1i64);
    }

    #[test]
    fn map_rating_converts_fields() {
        let rating = Rating {
            book_isbn: 42,
            star_rating: 4.5,
            num_ratings: 100,
        };
        let grpc = map_rating(rating);
        assert_eq!(grpc.book_isbn, "42");
        assert_eq!(grpc.star_rating, 4.5);
        assert_eq!(grpc.num_ratings, 100);
    }

    #[test]
    fn normalize_ranked_books_pagination_clamps_page_num() {
        assert_eq!(normalize_ranked_books_pagination(0, 10).0, 1);
        assert_eq!(normalize_ranked_books_pagination(-1, 10).0, 1);
    }

    #[test]
    fn normalize_ranked_books_pagination_clamps_page_size() {
        assert_eq!(normalize_ranked_books_pagination(1, 0).1, 1);
        assert_eq!(normalize_ranked_books_pagination(1, -1).1, 1);
        assert_eq!(normalize_ranked_books_pagination(1, 2000).1, 1000);
    }

    #[test]
    fn normalize_ranked_books_pagination_passes_through_valid() {
        assert_eq!(normalize_ranked_books_pagination(2, 50), (2, 50));
    }
}
