use sqlx::PgPool;
use tonic::{Request, Response, Status};
use redis::aio::ConnectionManager;

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
    pool: PgPool,
    redis: ConnectionManager,
}

impl RatingCatalogService {
    pub fn new(service_name: String, pool: PgPool, redis: ConnectionManager) -> Self {
        Self { service_name, pool, redis }
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

#[tonic::async_trait]
impl RatingCatalogGrpc for RatingCatalogService {
    // async fn health_check(
    //     &self,
    //     _request: Request<HealthCheckRequest>,
    // ) -> Result<Response<HealthCheckResponse>, Status> {
    //     let (service, status) = health_handler::health(&self.service_name);

    //     Ok(Response::new(HealthCheckResponse {
    //         service,
    //         status,
    //     }))
    // }

    async fn get_rating(
        &self,
        request: Request<GetRatingRequest>,
    ) -> Result<Response<GetRatingResponse>, Status> {
        let operation = "get_rating";
        let timer = metrics::start_timer("rating-catalog", operation);

        let result = async {
            let req = request.into_inner();

            let book_isbn = parse_book_isbn(&req.book_isbn)?;

            let rating = rating_handler::get_rating(&self.pool, book_isbn)
                .await
                .map_err(|err| Status::internal(err.to_string()))?
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

        let ratings: Vec<Rating> = rating_handler::get_ratings(&self.pool, &self.redis, req.page_number, req.page_size)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        
        let result = async {
            let req = request.into_inner();

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

            let rating = rating_handler::add_rating(
                &self.pool,
                book_isbn,
                payload.num_ratings,
                payload.star_rating,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

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

            let rating = rating_handler::update_rating(
                &self.pool,
                book_isbn,
                req.num_ratings,
                req.star_rating,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?
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

            let deleted = rating_handler::delete_rating(&self.pool, book_isbn)
                .await
                .map_err(|err| Status::internal(err.to_string()))?;

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
            let page_num = req.page_num.max(1);
            let page_size = req.page_size.max(1).min(1000);

            let ratings = rating_handler::get_books_by_rating(
                &self.pool,
                req.min_rating,
                page_num,
                page_size,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

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
            let page_num = req.page_num.max(1);
            let page_size = req.page_size.max(1).min(1000);

            let ratings = rating_handler::get_books_by_popularity(
                &self.pool,
                req.min_num_ratings,
                page_num,
                page_size,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

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
