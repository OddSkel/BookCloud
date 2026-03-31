use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::grpc::contracts::rating_catalog::{
    AddRatingRequest, AddRatingResponse, DeleteRatingRequest, DeleteRatingResponse,
    GetRatingsRequest, GetRatingsResponse, UpdateRatingRequest, UpdateRatingResponse,
};
use crate::handlers::rating_handler;
use crate::models::rating::Rating;

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

    async fn get_ratings(
        &self,
        request: Request<GetRatingsRequest>,
    ) -> Result<Response<GetRatingsResponse>, Status> {
        let req = request.into_inner();

        let ratings = rating_handler::get_ratings(&self.pool, req.page_number, req.page_size)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let response = GetRatingsResponse {
            ratings: ratings.into_iter().map(map_rating).collect(),
        };

        Ok(Response::new(response))
    }

    async fn add_rating(
        &self,
        request: Request<AddRatingRequest>,
    ) -> Result<Response<AddRatingResponse>, Status> {
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

    async fn update_rating(
        &self,
        request: Request<UpdateRatingRequest>,
    ) -> Result<Response<UpdateRatingResponse>, Status> {
        let req = request.into_inner();
        let book_isbn = parse_book_isbn(&req.book_isbn)?;

        let rating =
            rating_handler::update_rating(&self.pool, book_isbn, req.num_ratings, req.star_rating)
                .await
                .map_err(|err| Status::internal(err.to_string()))?
                .ok_or_else(|| Status::not_found("Rating not found"))?;

        Ok(Response::new(UpdateRatingResponse {
            ratings: vec![map_rating(rating)],
        }))
    }

    async fn delete_rating(
        &self,
        request: Request<DeleteRatingRequest>,
    ) -> Result<Response<DeleteRatingResponse>, Status> {
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
}
