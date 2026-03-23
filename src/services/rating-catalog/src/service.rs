use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::handlers::{
    health_handler, 
    rating_handler
};

use crate::grpc::contracts::{
    common::{HealthCheckRequest, HealthCheckResponse},
    // rating_catalog::{GetRatingRequest, GetRatingResponse, GetRatingBookRequest, GetRatingBookResponse, CreateRatingRequest, CreateRatingResponse, UpdateRatingRequest, UpdateRatingResponse, DeleteRatingRequest, DeleteRatingResponse},
    rating_catalog::{GetRatingRequest, GetRatingResponse},
    rating_catalog::rating_catalog_grpc_server::RatingCatalogGrpc,
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
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let (service, status) = health_handler::health(&self.service_name);

        Ok(Response::new(HealthCheckResponse {
            service,
            status,
        }))
    }

    async fn get_rating(
        &self,
        request: Request<GetRatingRequest>,
    ) -> Result<Response<GetRatingResponse>, Status> {
        let req = request.into_inner();

        let rating_id: i32 = req
            .rating_id
            .parse()
            .map_err(|_| Status::invalid_argument("Invalid rating_id"))?;

        let rating = rating_handler::get_rating(&self.pool, rating_id)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        Ok(Response::new(GetRatingResponse {
            status: "ok".to_string(),
            // id: rating.id.to_string(),
            // book_id: rating.book_id,
            // evaluation: rating.evaluation,
            // critic: rating.critic,
        }))
    }

    // async fn get_rating_book(
    //     &self,
    //     _request: Request<GetRatingRequest>,
    // ) -> Result<Response<GetRatingResponse>, Status> {
    //     Ok(Response::new(GetRatingResponse {
    //         service: self.service_name.clone(),
    //         status: "ok".to_string(),
    //     }))
    // }

    // async fn create_rating(
    //     &self,
    //     _request: Request<CreateRatingRequest>,
    // ) -> Result<Response<CreateRatingResponse>, Status> {
    //     Ok(Response::new(CreateRatingResponse {
    //         service: self.service_name.clone(),
    //         status: "ok".to_string(),
    //     }))
    // }


    // async fn update_rating(
    //     &self,
    //     _request: Request<UpdateRatingRequest>,
    // ) -> Result<Response<UpdateRatingResponse>, Status> {
    //     Ok(Response::new(UpdateRatingResponse {
    //         service: self.service_name.clone(),
    //         status: "ok".to_string(),
    //     }))
    // }
    

    // async fn delete_rating(
    //     &self,
    //     _request: Request<DeleteRatingRequest>,
    // ) -> Result<Response<DeleteRatingResponse>, Status> {
    //     Ok(Response::new(DeleteRatingResponse {
    //         service: self.service_name.clone(),
    //         status: "ok".to_string(),
    //     }))
    // }

}
