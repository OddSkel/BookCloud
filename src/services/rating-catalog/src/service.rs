use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::contracts::rating_catalog::{GetRatingsRequest, GetRatingsResponse};
use crate::handlers::rating_handler;

use crate::grpc::contracts::{
    rating_catalog::rating_catalog_grpc_server::RatingCatalogGrpc,
    rating_catalog::{GetRatingRequest, GetRatingResponse, Rating as GrpcRating},
};

pub struct RatingCatalogService {
    service_name: String,
    pool: PgPool,
}

impl RatingCatalogService {
    pub fn new(service_name: String, pool: PgPool) -> Self {
        Self { service_name, pool }
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
        let req = request.into_inner();

        let book_isbn: i64 = req
            .book_isbn
            .parse()
            .map_err(|_| Status::invalid_argument("Invalid book_isbn"))?;

        let rating = rating_handler::get_rating(&self.pool, book_isbn)
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .ok_or_else(|| Status::not_found("Rating not found"))?;

        let response = GetRatingResponse {
            rating: Some(GrpcRating {
                book_isbn: rating.book_isbn.to_string(),
                star_rating: rating.star_rating,
                num_ratings: rating.num_ratings,
            }),
        };

        Ok(Response::new(response))
    }

    async fn get_ratings(
        &self,
        request: Request<GetRatingsRequest>,
    ) -> Result<Response<GetRatingsResponse>, Status> {
        let req = request.into_inner();

        let ratings = rating_handler::get_ratings(&self.pool, req.page_number, req.page_size)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let response = GetRatingsResponse {
            ratings: ratings
                .into_iter()
                .map(|rating| GrpcRating {
                    book_isbn: rating.book_isbn.to_string(),
                    star_rating: rating.star_rating,
                    num_ratings: rating.num_ratings,
                })
                .collect(),
        };

        Ok(Response::new(response))
    }
}
