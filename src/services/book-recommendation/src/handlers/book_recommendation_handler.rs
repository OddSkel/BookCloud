use sqlx::PgPool;
use tonic::{Request, Status};
use tonic::transport::Error as TonicError;

use crate::grpc::contracts::book_recommendation::{Query, BookRecommendationResponse, Book as Proto_Book};
use crate::grpc::contracts::book_catalog::{GetBookRequest, book_catalog_grpc_client::BookCatalogGrpcClient};
use crate::grpc::contracts::genre_service::{GetBooksByGenreRequest, GetBooksByGenreResponse, genre_analysis_grpc_client::GenreAnalysisGrpcClient};
use crate::grpc::contracts::rating_catalog::{GetBooksByRatingRequest, GetBooksByRatingResponse, BookRatingInfo, GetBooksByPopularityRequest, GetBooksByPopularityResponse, rating_catalog_grpc_client::RatingCatalogGrpcClient};

pub async fn book_recommendation(
    pool: &PgPool,
    params: Query,
    genre_analysis_grpc_url: &str,
    book_catalog_grpc_url: &str,
    rating_catalog_grpc_url: &str,
) -> Result<BookRecommendationResponse, String> {

    let mut book_isbns: Vec<i64> = Vec::new();

    if let Some(genre) = params.genre.as_deref().filter(|s| !s.trim().is_empty()) {
        let mut genre_client: GenreAnalysisGrpcClient<tonic::transport::Channel> = GenreAnalysisGrpcClient::connect(genre_analysis_grpc_url.to_string())
            .await
            .map_err(|e: TonicError| e.to_string())?;

        let genre_response = genre_client
            .get_books_by_genre(Request::new(GetBooksByGenreRequest {
                genre_name: genre.to_string(),
                page_num: 1,
                page_size: 1000,
            }))
            .await
            .map_err(|e: Status| e.to_string())
            .map(|r| r.into_inner())?;

        for item in genre_response.items {
            book_isbns.push(item.isbn);
        }
    }

    if let Some(rating) = params.rating {
        let mut rating_client: RatingCatalogGrpcClient<tonic::transport::Channel> = RatingCatalogGrpcClient::connect(rating_catalog_grpc_url.to_string())
            .await
            .map_err(|e: TonicError| e.to_string())?;

        let rating_response = rating_client
            .get_books_by_rating(Request::new(GetBooksByRatingRequest {
                min_rating: rating,
                page_num: 1,
                page_size: 1000,
            }))
            .await
            .map_err(|e: Status| e.to_string())
            .map(|r| r.into_inner())?;

        let rating_isbns: Vec<i64> = rating_response.items.iter()
            .filter_map(|i| i.book_isbn.parse().ok())
            .collect();

        if book_isbns.is_empty() {
            book_isbns = rating_isbns;
        } else {
            book_isbns.retain(|isbn| rating_isbns.contains(isbn));
        }
    }

    if let Some(popularity) = params.popularity.filter(|&p| p > 0.0).map(|p| p as i32) {
        let mut rating_client: RatingCatalogGrpcClient<tonic::transport::Channel> = RatingCatalogGrpcClient::connect(rating_catalog_grpc_url.to_string())
            .await
            .map_err(|e: TonicError| e.to_string())?;

        let pop_response = rating_client
            .get_books_by_popularity(Request::new(GetBooksByPopularityRequest {
                min_num_ratings: popularity,
                page_num: 1,
                page_size: 1000,
            }))
            .await
            .map_err(|e: Status| e.to_string())
            .map(|r| r.into_inner())?;

        let pop_isbns: Vec<i64> = pop_response.items.iter()
            .filter_map(|i| i.book_isbn.parse().ok())
            .collect();

        if book_isbns.is_empty() {
            book_isbns = pop_isbns;
        } else {
            book_isbns.retain(|isbn| pop_isbns.contains(isbn));
        }
    }

    if book_isbns.is_empty() {
        let query = "SELECT isbn FROM books";
        let rows: Vec<(i64,)> = sqlx::query_as(query)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
        book_isbns = rows.into_iter().map(|(isbn,)| isbn).collect();
    }

    let mut books = Vec::new();
    let mut book_client = BookCatalogGrpcClient::connect(book_catalog_grpc_url.to_string())
        .await
        .map_err(|e: TonicError| e.to_string())?;

    for isbn in book_isbns {
        match book_client
            .get_book(Request::new(GetBookRequest { isbn }))
            .await
        {
            Ok(response) => {
                if let Some(book) = response.into_inner().book {
                    books.push(Proto_Book {
                        isbn: book.isbn,
                        name: book.name,
                        url: book.url,
                        pub_year: book.pub_year,
                    });
                }
            }
            Err(_) => continue,
        }
    }

    Ok(BookRecommendationResponse { books })
}