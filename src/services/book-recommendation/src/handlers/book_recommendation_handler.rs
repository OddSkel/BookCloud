use tonic::{Request, Status};

use crate::grpc::contracts::book_catalog::{
    GetBookRequest, book_catalog_grpc_client::BookCatalogGrpcClient,
};
use crate::grpc::contracts::book_recommendation::{
    Book as Proto_Book, BookRecommendationResponse, Query,
};
use crate::grpc::contracts::genre_service::{
    GetBooksByGenreRequest, genre_analysis_grpc_client::GenreAnalysisGrpcClient,
};
use crate::grpc::contracts::rating_catalog::{
    GetBooksByPopularityRequest, GetBooksByRatingRequest,
    rating_catalog_grpc_client::RatingCatalogGrpcClient,
};

const MAX_RECOMMENDATION_RESULTS: usize = 50;

pub async fn book_recommendation(
    params: Query,
    genre_client: &mut GenreAnalysisGrpcClient<tonic::transport::Channel>,
    book_catalog_client: &mut BookCatalogGrpcClient<tonic::transport::Channel>,
    rating_catalog_client: &mut RatingCatalogGrpcClient<tonic::transport::Channel>,
) -> Result<BookRecommendationResponse, String> {
        let mut book_isbns: Vec<i64> = Vec::new();

    // Step 1: Filter by genre
    if let Some(genre) = params.genre.as_deref().filter(|s| !s.trim().is_empty()) {
        let genre_response = genre_client
            .get_books_by_genre(Request::new(GetBooksByGenreRequest {
                genre_name: genre.to_string(),
                page_num: 1,
                page_size: 1000,
            }))
            .await
            .map_err(|e: Status| e.to_string())
            .map(|r| r.into_inner())?;

        book_isbns = genre_response.items.iter().map(|i| i.isbn).collect();
    }

    // Step 2: Filter by rating and popularity — reuse passed-in client
    if params.rating.is_some() || params.popularity.is_some() {
        if let Some(rating) = params.rating {
            let rating_response = rating_catalog_client
                .get_books_by_rating(Request::new(GetBooksByRatingRequest {
                    min_rating: rating,
                    page_num: 1,
                    page_size: 1000,
                }))
                .await
                .map_err(|e: Status| e.to_string())
                .map(|r| r.into_inner())?;

            let rating_isbns: Vec<i64> = rating_response
                .items
                .iter()
                .filter_map(|i| i.book_isbn.parse().ok())
                .collect();

            if book_isbns.is_empty() {
                book_isbns = rating_isbns;
            } else {
                book_isbns.retain(|isbn| rating_isbns.contains(isbn));
            }
        }

        if let Some(popularity) = params.popularity.filter(|&p| p > 0.0).map(|p| p as i32) {
            let pop_response = rating_catalog_client
                .get_books_by_popularity(Request::new(GetBooksByPopularityRequest {
                    min_num_ratings: popularity,
                    page_num: 1,
                    page_size: 1000,
                }))
                .await
                .map_err(|e: Status| e.to_string())
                .map(|r| r.into_inner())?;

            let pop_isbns: Vec<i64> = pop_response
                .items
                .iter()
                .filter_map(|i| i.book_isbn.parse().ok())
                .collect();

            if book_isbns.is_empty() {
                book_isbns = pop_isbns;
            } else {
                book_isbns.retain(|isbn| pop_isbns.contains(isbn));
            }
        }
    }

    // Step 3: Fallback — return empty if no filters matched any books
    if book_isbns.is_empty() {
        return Ok(BookRecommendationResponse { books: vec![] });
    }

    book_isbns.truncate(20);

    // Step 4: Fetch book details using passed-in client
    let mut books = Vec::new();
    for isbn in book_isbns.into_iter().take(MAX_RECOMMENDATION_RESULTS) {
        match book_catalog_client
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
