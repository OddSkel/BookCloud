use serde::Serialize;
use std::{collections::HashMap, env};
use tonic::Request;

pub mod contracts {
    pub mod common {
        tonic::include_proto!("gateway.common");
    }

    pub mod book_catalog {
        tonic::include_proto!("gateway.bookcatalog");
    }

    pub mod author_catalog {
        tonic::include_proto!("gateway.authorcatalog");
    }

    pub mod rating_catalog {
        tonic::include_proto!("gateway.ratingcatalog");
    }
}

use contracts::{
    author_catalog::author_catalog_grpc_client::AuthorCatalogGrpcClient,
    book_catalog::book_catalog_grpc_client::BookCatalogGrpcClient,
    common::HealthCheckRequest,
    rating_catalog::rating_catalog_grpc_client::RatingCatalogGrpcClient,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatalogService {
    BookCatalog,
    AuthorCatalog,
    RatingCatalog,
}

impl CatalogService {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::BookCatalog => "bookCatalog",
            Self::AuthorCatalog => "authorCatalog",
            Self::RatingCatalog => "ratingCatalog",
        }
    }

    pub const fn env_var(self) -> &'static str {
        match self {
            Self::BookCatalog => "BOOK_CATALOG_GRPC_URL",
            Self::AuthorCatalog => "AUTHOR_CATALOG_GRPC_URL",
            Self::RatingCatalog => "RATING_CATALOG_GRPC_URL",
        }
    }

    pub const fn default_uri(self) -> &'static str {
        match self {
            Self::BookCatalog => "http://book-catalog:50051",
            Self::AuthorCatalog => "http://author-catalog:50052",
            Self::RatingCatalog => "http://rating-catalog:50053",
        }
    }
}

#[derive(Clone)]
pub struct GrpcRegistry {
    endpoints: HashMap<CatalogService, String>,
}

impl GrpcRegistry {
    pub fn from_env() -> Self {
        let endpoints = [
            CatalogService::BookCatalog,
            CatalogService::AuthorCatalog,
            CatalogService::RatingCatalog,
        ]
        .into_iter()
        .map(|service| {
            let endpoint = env::var(service.env_var())
                .unwrap_or_else(|_| service.default_uri().to_string());
            (service, endpoint)
        })
        .collect();

        Self { endpoints }
    }

    pub async fn health_status(&self, service: CatalogService) -> GrpcServiceStatus {
        let endpoint = self.endpoint(service).to_string();

        match health_check(service, &endpoint).await {
            Ok(response) => GrpcServiceStatus {
                service: service.slug().to_string(),
                endpoint,
                connected: true,
                upstream_service: Some(response.service),
                health_status: Some(response.status),
                message: "HealthCheck RPC completed successfully".to_string(),
            },
            Err(error) => GrpcServiceStatus {
                service: service.slug().to_string(),
                endpoint,
                connected: false,
                upstream_service: None,
                health_status: None,
                message: error,
            },
        }
    }

    fn endpoint(&self, service: CatalogService) -> &str {
        self.endpoints
            .get(&service)
            .map(String::as_str)
            .expect("gRPC service endpoint must be configured")
    }
}

#[derive(Serialize)]
pub struct GrpcServiceStatus {
    pub service: String,
    pub endpoint: String,
    pub connected: bool,
    pub upstream_service: Option<String>,
    pub health_status: Option<String>,
    pub message: String,
}

async fn health_check(
    service: CatalogService,
    endpoint: &str,
) -> Result<contracts::common::HealthCheckResponse, String> {
    match service {
        CatalogService::BookCatalog => book_catalog_health(endpoint).await,
        CatalogService::AuthorCatalog => author_catalog_health(endpoint).await,
        CatalogService::RatingCatalog => rating_catalog_health(endpoint).await,
    }
}

async fn book_catalog_health(
    endpoint: &str,
) -> Result<contracts::common::HealthCheckResponse, String> {
    let mut client = BookCatalogGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|error| error.to_string())?;

    client
        .health_check(Request::new(HealthCheckRequest {}))
        .await
        .map(|response| response.into_inner())
        .map_err(|error| error.to_string())
}

async fn author_catalog_health(
    endpoint: &str,
) -> Result<contracts::common::HealthCheckResponse, String> {
    let mut client = AuthorCatalogGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|error| error.to_string())?;

    client
        .health_check(Request::new(HealthCheckRequest {}))
        .await
        .map(|response| response.into_inner())
        .map_err(|error| error.to_string())
}

async fn rating_catalog_health(
    endpoint: &str,
) -> Result<contracts::common::HealthCheckResponse, String> {
    let mut client = RatingCatalogGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|error| error.to_string())?;

    client
        .health_check(Request::new(HealthCheckRequest {}))
        .await
        .map(|response| response.into_inner())
        .map_err(|error| error.to_string())
}
