use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};
use tonic::Request;

const COMPARE_GRPC_MESSAGE_SIZE_LIMIT: usize = 128 * 1024 * 1024;
const RATING_GRPC_MESSAGE_SIZE_LIMIT: usize = 128 * 1024 * 1024;

// Generated proto contracts

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
    pub mod compare_service {
        tonic::include_proto!("gateway.compare");
    }
}

use contracts::{
    author_catalog::{
        AddAuthorRequest, AuthorAdd, DeleteAuthorRequest, GetAuthorRequest, GetAuthorsRequest,
        UpdateAuthorRequest, author_catalog_grpc_client::AuthorCatalogGrpcClient,
    },
    book_catalog::{
        AddBookRequest, BookAdd, DeleteBookRequest, GetBookRequest, GetBooksRequest,
        UpdateBookRequest, book_catalog_grpc_client::BookCatalogGrpcClient,
    },
    common::HealthCheckRequest,
    compare_service::{
        CompareFilters, GetCorrelationRequest, GetErasRequest, GetHiddenGemsRequest,
        GetPopularLowRatedRequest, GetPublishingGrowthRequest, JsonPayloadResponse,
        compare_service_grpc_client::CompareServiceGrpcClient,
    },
    rating_catalog::{
        AddRatingRequest, DeleteRatingRequest, GetRatingRequest, GetRatingsRequest, RatingAdd,
        UpdateRatingRequest, rating_catalog_grpc_client::RatingCatalogGrpcClient,
    },
};

// Domain models (HTTP ↔ gRPC bridge types)

#[derive(Serialize, Deserialize, Clone)]
pub struct BookModel {
    pub id: String,
    pub name: String,
    pub isbn: String,
    pub url: String,
    pub summary: Option<String>,
    pub pub_year: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BookAddPayload {
    pub name: String,
    pub isbn: String,
    pub url: String,
    pub summary: Option<String>,
    pub pub_year: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthorModel {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub year_born: i32,
    pub year_death: Option<i32>,
    pub books_published: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthorAddPayload {
    pub name: String,
    pub gender: String,
    pub year_born: i32,
    pub year_death: Option<i32>,
    pub books_published: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RatingModel {
    pub book_isbn: String,
    pub num_ratings: i64,
    pub star_rating: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RatingAddPayload {
    pub num_ratings: i64,
    pub star_rating: f64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CompareFiltersPayload {
    pub author_name: Option<String>,
    pub author_id: Option<i64>,
    pub genre_name: Vec<String>,
    pub genre_id: Vec<i64>,
    pub book_name: Option<String>,
    pub book_isbn: Option<i64>,
    pub pub_year_from: Option<i32>,
    pub pub_year_to: Option<i32>,
    pub pub_year: Option<i32>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub min_num_ratings: Option<i64>,
    pub max_num_ratings: Option<i64>,
    pub min_star_rating: Option<f64>,
    pub max_star_rating: Option<f64>,
    pub method: Option<String>,
    pub classic_threshold: Option<i32>,
    pub modern_threshold: Option<i32>,
}

// CatalogService

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatalogService {
    BookCatalog,
    AuthorCatalog,
    RatingCatalog,
    CompareService,
}

impl CatalogService {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::BookCatalog => "bookCatalog",
            Self::AuthorCatalog => "authorCatalog",
            Self::RatingCatalog => "ratingCatalog",
            Self::CompareService => "compareService",
        }
    }
    pub const fn env_var(self) -> &'static str {
        match self {
            Self::BookCatalog => "BOOK_CATALOG_GRPC_URL",
            Self::AuthorCatalog => "AUTHOR_CATALOG_GRPC_URL",
            Self::RatingCatalog => "RATING_CATALOG_GRPC_URL",
            Self::CompareService => "COMPARE_SERVICE_GRPC_URL",
        }
    }
    pub const fn default_uri(self) -> &'static str {
        match self {
            Self::BookCatalog => "http://book-catalog:50051",
            Self::AuthorCatalog => "http://author-catalog:50052",
            Self::RatingCatalog => "http://rating-catalog:50053",
            Self::CompareService => "http://compare-service:50054",
        }
    }
}

// GrpcRegistry

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
            CatalogService::CompareService,
        ]
        .into_iter()
        .map(|svc| {
            let url = env::var(svc.env_var()).unwrap_or_else(|_| svc.default_uri().to_string());
            (svc, url)
        })
        .collect();
        Self { endpoints }
    }

    fn endpoint(&self, service: CatalogService) -> &str {
        self.endpoints
            .get(&service)
            .map(String::as_str)
            .expect("gRPC endpoint must be configured")
    }

    // Health

    pub async fn health_status(&self, service: CatalogService) -> GrpcServiceStatus {
        let endpoint = self.endpoint(service).to_string();
        match health_check(service, &endpoint).await {
            Ok(r) => GrpcServiceStatus {
                service: service.slug().to_string(),
                endpoint,
                connected: true,
                upstream_service: Some(r.service),
                health_status: Some(r.status),
                message: "HealthCheck RPC completed successfully".to_string(),
            },
            Err(e) => GrpcServiceStatus {
                service: service.slug().to_string(),
                endpoint,
                connected: false,
                upstream_service: None,
                health_status: None,
                message: e,
            },
        }
    }

    // Books

    pub async fn get_books(
        &self,
        page_num: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<Vec<BookModel>, String> {
        let mut c = book_client(self.endpoint(CatalogService::BookCatalog)).await?;
        let page_num = page_num.unwrap_or(1).max(1);
        let page_size = page_size.unwrap_or(10).max(1);

        let r = c
            .get_books(Request::new(GetBooksRequest {
                page_num,
                page_size,
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        Ok(r.books.into_iter().map(book_to_model).collect())
    }

    pub async fn get_book(&self, book_isbn: &str) -> Result<BookModel, String> {
        let mut c = book_client(self.endpoint(CatalogService::BookCatalog)).await?;
        let isbn = book_isbn
            .parse::<i64>()
            .map_err(|_| "Invalid ISBN".to_string())?;
        let r = c
            .get_book(Request::new(GetBookRequest { isbn }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        r.book
            .map(book_to_model)
            .ok_or_else(|| "Book not found".to_string())
    }

    pub async fn add_book(&self, p: BookAddPayload) -> Result<BookModel, String> {
        let mut c = book_client(self.endpoint(CatalogService::BookCatalog)).await?;
        let r = c
            .add_book(Request::new(AddBookRequest {
                book: Some(BookAdd {
                    isbn: p
                        .isbn
                        .parse::<i64>()
                        .map_err(|_| "Invalid ISBN".to_string())?,
                    name: p.name,
                    url: p.url,
                    summary_clean: p.summary.unwrap_or_default(),
                    pub_year: p.pub_year,
                }),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        r.book
            .map(book_to_model)
            .ok_or_else(|| "Failed to create book".to_string())
    }

    pub async fn update_book(
        &self,
        isbn_header: Option<String>,
        p: BookAddPayload,
    ) -> Result<Vec<BookModel>, String> {
        let mut c = book_client(self.endpoint(CatalogService::BookCatalog)).await?;
        let isbn_str = isbn_header.ok_or_else(|| "ISBN header missing".to_string())?;
        let isbn = isbn_str
            .parse::<i64>()
            .map_err(|_| "Invalid ISBN".to_string())?;

        let r = c
            .update_book(Request::new(UpdateBookRequest {
                isbn,
                book: Some(BookAdd {
                    isbn,
                    name: p.name,
                    url: p.url,
                    summary_clean: p.summary.unwrap_or_default(),
                    pub_year: p.pub_year,
                }),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        let updated = r
            .book
            .map(book_to_model)
            .ok_or_else(|| "Failed to update book".to_string())?;
        Ok(vec![updated])
    }

    pub async fn delete_book(&self, book_isbn: &str) -> Result<(), String> {
        let mut c = book_client(self.endpoint(CatalogService::BookCatalog)).await?;
        let isbn = book_isbn
            .parse::<i64>()
            .map_err(|_| "Invalid ISBN".to_string())?;
        c.delete_book(Request::new(DeleteBookRequest { isbn }))
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    // Authors

    pub async fn get_authors(&self) -> Result<Vec<AuthorModel>, String> {
        let mut c = author_client(self.endpoint(CatalogService::AuthorCatalog)).await?;
        let r = c
            .get_authors(Request::new(GetAuthorsRequest {}))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        Ok(r.authors.into_iter().map(author_to_model).collect())
    }

    pub async fn get_author(&self, author_id: &str) -> Result<AuthorModel, String> {
        let mut c = author_client(self.endpoint(CatalogService::AuthorCatalog)).await?;
        let r = c
            .get_author(Request::new(GetAuthorRequest {
                author_id: author_id.to_string(),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        r.author
            .map(author_to_model)
            .ok_or_else(|| "Author not found".to_string())
    }

    pub async fn add_author(&self, p: AuthorAddPayload) -> Result<AuthorModel, String> {
        let mut c = author_client(self.endpoint(CatalogService::AuthorCatalog)).await?;
        let r = c
            .add_author(Request::new(AddAuthorRequest {
                author: Some(AuthorAdd {
                    name: p.name,
                    gender: p.gender,
                    year_born: p.year_born,
                    year_death: p.year_death,
                    books_published: p.books_published,
                }),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        r.author
            .map(author_to_model)
            .ok_or_else(|| "Failed to create author".to_string())
    }

    pub async fn update_author(&self, name: Option<String>) -> Result<Vec<AuthorModel>, String> {
        let mut c = author_client(self.endpoint(CatalogService::AuthorCatalog)).await?;
        let r = c
            .update_author(Request::new(UpdateAuthorRequest { name }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        Ok(r.authors.into_iter().map(author_to_model).collect())
    }

    pub async fn delete_author(&self, author_id: &str) -> Result<(), String> {
        let mut c = author_client(self.endpoint(CatalogService::AuthorCatalog)).await?;
        c.delete_author(Request::new(DeleteAuthorRequest {
            author_id: author_id.to_string(),
        }))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    // Ratings

    pub async fn get_ratings(
        &self,
        page_number: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Vec<RatingModel>, String> {
        let mut c = rating_client(self.endpoint(CatalogService::RatingCatalog)).await?;

        let r = c
            .get_ratings(Request::new(GetRatingsRequest {
                page_number,
                page_size,
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        Ok(r.ratings.into_iter().map(rating_to_model).collect())
    }

    pub async fn get_rating(&self, book_isbn: &str) -> Result<RatingModel, String> {
        let mut c = rating_client(self.endpoint(CatalogService::RatingCatalog)).await?;
        let r = c
            .get_rating(Request::new(GetRatingRequest {
                book_isbn: book_isbn.to_string(),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;
        r.rating
            .map(rating_to_model)
            .ok_or_else(|| "Rating not found".to_string())
    }

    pub async fn add_rating(
        &self,
        book_isbn: &str,
        p: RatingAddPayload,
    ) -> Result<RatingModel, String> {
        let mut c = rating_client(self.endpoint(CatalogService::RatingCatalog)).await?;
        let r = c
            .add_rating(Request::new(AddRatingRequest {
                book_isbn: book_isbn.to_string(),
                rating: Some(RatingAdd {
                    num_ratings: p.num_ratings,
                    star_rating: p.star_rating,
                }),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        r.rating
            .map(rating_to_model)
            .ok_or_else(|| "Failed to create rating".to_string())
    }

    pub async fn update_rating(
        &self,
        book_isbn: &str,
        num_ratings: i64,
        star_rating: f64,
    ) -> Result<Vec<RatingModel>, String> {
        let mut c = rating_client(self.endpoint(CatalogService::RatingCatalog)).await?;
        let r = c
            .update_rating(Request::new(UpdateRatingRequest {
                book_isbn: book_isbn.to_string(),
                num_ratings,
                star_rating,
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        Ok(r.ratings.into_iter().map(rating_to_model).collect())
    }

    pub async fn delete_rating(&self, book_isbn: &str) -> Result<(), String> {
        let mut c = rating_client(self.endpoint(CatalogService::RatingCatalog)).await?;
        c.delete_rating(Request::new(DeleteRatingRequest {
            book_isbn: book_isbn.to_string(),
        }))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    // Compare

    pub async fn get_popular_low_rated(
        &self,
        filters: CompareFiltersPayload,
    ) -> Result<serde_json::Value, String> {
        let mut c = compare_client(self.endpoint(CatalogService::CompareService)).await?;
        let response = c
            .get_popular_low_rated(Request::new(GetPopularLowRatedRequest {
                filters: Some(compare_filters_to_proto(filters)),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        compare_json_to_value(response)
    }

    pub async fn get_hidden_gems(
        &self,
        filters: CompareFiltersPayload,
    ) -> Result<serde_json::Value, String> {
        let mut c = compare_client(self.endpoint(CatalogService::CompareService)).await?;
        let response = c
            .get_hidden_gems(Request::new(GetHiddenGemsRequest {
                filters: Some(compare_filters_to_proto(filters)),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        compare_json_to_value(response)
    }

    pub async fn get_correlation(
        &self,
        filters: CompareFiltersPayload,
    ) -> Result<serde_json::Value, String> {
        let mut c = compare_client(self.endpoint(CatalogService::CompareService)).await?;
        let response = c
            .get_correlation(Request::new(GetCorrelationRequest {
                filters: Some(compare_filters_to_proto(filters)),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        compare_json_to_value(response)
    }

    pub async fn get_publishing_growth(
        &self,
        filters: CompareFiltersPayload,
    ) -> Result<serde_json::Value, String> {
        let mut c = compare_client(self.endpoint(CatalogService::CompareService)).await?;
        let response = c
            .get_publishing_growth(Request::new(GetPublishingGrowthRequest {
                filters: Some(compare_filters_to_proto(filters)),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        compare_json_to_value(response)
    }

    pub async fn get_eras(
        &self,
        filters: CompareFiltersPayload,
    ) -> Result<serde_json::Value, String> {
        let mut c = compare_client(self.endpoint(CatalogService::CompareService)).await?;
        let response = c
            .get_eras(Request::new(GetErasRequest {
                filters: Some(compare_filters_to_proto(filters)),
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        compare_json_to_value(response)
    }
}

// GrpcServiceStatus

#[derive(Serialize)]
pub struct GrpcServiceStatus {
    pub service: String,
    pub endpoint: String,
    pub connected: bool,
    pub upstream_service: Option<String>,
    pub health_status: Option<String>,
    pub message: String,
}

// Private client constructors

async fn book_client(
    endpoint: &str,
) -> Result<BookCatalogGrpcClient<tonic::transport::Channel>, String> {
    BookCatalogGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|e| e.to_string())
}

async fn author_client(
    endpoint: &str,
) -> Result<AuthorCatalogGrpcClient<tonic::transport::Channel>, String> {
    AuthorCatalogGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|e| e.to_string())
}

async fn rating_client(
    endpoint: &str,
) -> Result<RatingCatalogGrpcClient<tonic::transport::Channel>, String> {
    let client = RatingCatalogGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|e| e.to_string())?;

    Ok(client
        .max_decoding_message_size(RATING_GRPC_MESSAGE_SIZE_LIMIT)
        .max_encoding_message_size(RATING_GRPC_MESSAGE_SIZE_LIMIT))
}

async fn compare_client(
    endpoint: &str,
) -> Result<CompareServiceGrpcClient<tonic::transport::Channel>, String> {
    let client = CompareServiceGrpcClient::connect(endpoint.to_string())
        .await
        .map_err(|e| e.to_string())?;

    Ok(client
        .max_decoding_message_size(COMPARE_GRPC_MESSAGE_SIZE_LIMIT)
        .max_encoding_message_size(COMPARE_GRPC_MESSAGE_SIZE_LIMIT))
}

// Health check helpers

async fn health_check(
    service: CatalogService,
    endpoint: &str,
) -> Result<contracts::common::HealthCheckResponse, String> {
    match service {
        CatalogService::BookCatalog => book_health(endpoint).await,
        CatalogService::AuthorCatalog => author_health(endpoint).await,
        CatalogService::RatingCatalog => rating_health(endpoint).await,
        CatalogService::CompareService => compare_health(endpoint).await,
    }
}

async fn book_health(endpoint: &str) -> Result<contracts::common::HealthCheckResponse, String> {
    book_client(endpoint)
        .await?
        .health_check(Request::new(HealthCheckRequest {}))
        .await
        .map(|r| r.into_inner())
        .map_err(|e| e.to_string())
}

async fn author_health(endpoint: &str) -> Result<contracts::common::HealthCheckResponse, String> {
    author_client(endpoint)
        .await?
        .health_check(Request::new(HealthCheckRequest {}))
        .await
        .map(|r| r.into_inner())
        .map_err(|e| e.to_string())
}

async fn rating_health(_endpoint: &str) -> Result<contracts::common::HealthCheckResponse, String> {
    Err("RatingCatalog does not expose health_check RPC".to_string())
}

async fn compare_health(_endpoint: &str) -> Result<contracts::common::HealthCheckResponse, String> {
    Err("CompareService does not expose health_check RPC".to_string())
}

// Proto → Model converters

fn book_to_model(b: contracts::book_catalog::Book) -> BookModel {
    BookModel {
        id: b.isbn.to_string(),
        name: b.name,
        isbn: b.isbn.to_string(),
        url: b.url,
        summary: if b.summary_clean.is_empty() {
            None
        } else {
            Some(b.summary_clean)
        },
        pub_year: b.pub_year,
    }
}

fn author_to_model(a: contracts::author_catalog::Author) -> AuthorModel {
    AuthorModel {
        id: a.id,
        name: a.name,
        gender: a.gender,
        year_born: a.year_born,
        year_death: a.year_death,
        books_published: a.books_published,
    }
}

fn rating_to_model(r: contracts::rating_catalog::Rating) -> RatingModel {
    RatingModel {
        book_isbn: r.book_isbn,
        num_ratings: r.num_ratings,
        star_rating: r.star_rating,
    }
}

fn compare_filters_to_proto(payload: CompareFiltersPayload) -> CompareFilters {
    CompareFilters {
        author_name: payload.author_name,
        author_id: payload.author_id,
        genre_name: payload.genre_name,
        genre_id: payload.genre_id,
        book_name: payload.book_name,
        book_isbn: payload.book_isbn,
        pub_year_from: payload.pub_year_from,
        pub_year_to: payload.pub_year_to,
        pub_year: payload.pub_year,
        page: payload.page,
        page_size: payload.page_size,
        min_num_ratings: payload.min_num_ratings,
        max_num_ratings: payload.max_num_ratings,
        min_star_rating: payload.min_star_rating,
        max_star_rating: payload.max_star_rating,
        method: payload.method,
        classic_threshold: payload.classic_threshold,
        modern_threshold: payload.modern_threshold,
    }
}

fn compare_json_to_value(response: JsonPayloadResponse) -> Result<serde_json::Value, String> {
    serde_json::from_str(&response.json_payload).map_err(|e| e.to_string())
}
