use sqlx::PgPool;

use crate::db::rating::{
    add_rating_query, delete_rating_query, get_rating_by_id_query, get_ratings_query,
    update_rating_query,
};
use crate::models::rating::Rating;

pub async fn get_rating(pool: &PgPool, book_isbn: i64) -> Result<Option<Rating>, sqlx::Error> {
    get_rating_by_id_query(pool, book_isbn).await
}

pub async fn get_ratings(
    pool: &PgPool,
    page_number: Option<i64>,
    page_size: Option<i64>,
) -> Result<Vec<Rating>, sqlx::Error> {
    get_ratings_query(pool, page_number, page_size).await
}

pub async fn add_rating(
    pool: &PgPool,
    book_isbn: i64,
    num_ratings: i64,
    star_rating: f64,
) -> Result<Rating, sqlx::Error> {
    add_rating_query(pool, book_isbn, num_ratings, star_rating).await
}

pub async fn update_rating(
    pool: &PgPool,
    book_isbn: i64,
    num_ratings: i64,
    star_rating: f64,
) -> Result<Option<Rating>, sqlx::Error> {
    update_rating_query(pool, book_isbn, num_ratings, star_rating).await
}

pub async fn delete_rating(pool: &PgPool, book_isbn: i64) -> Result<bool, sqlx::Error> {
    delete_rating_query(pool, book_isbn).await
}
