use anyhow::Result;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::db::rating::{
    add_rating_query, delete_rating_query, get_rating_by_id_query, get_ratings_query,
    update_rating_query,
};
use crate::models::rating::Rating;

const BOOKS_BY_RATING_QUERY: &str = "SELECT book_isbn, num_ratings, star_rating FROM rating WHERE star_rating >= $1 ORDER BY star_rating DESC LIMIT $2 OFFSET $3";
const BOOKS_BY_POPULARITY_QUERY: &str = "SELECT book_isbn, num_ratings, star_rating FROM rating WHERE num_ratings >= $1 ORDER BY num_ratings DESC LIMIT $2 OFFSET $3";

pub async fn get_rating(pool: &PgPool, book_isbn: i64) -> Result<Option<Rating>, sqlx::Error> {
    get_rating_by_id_query(pool, book_isbn).await
}

pub async fn get_ratings(
    pool: &PgPool,
    redis: &ConnectionManager,
    page_number: Option<i64>,
    page_size: Option<i64>,
    cache_ttl_seconds: u64,
) -> Result<Vec<Rating>> {
    get_ratings_query(pool, redis, page_number, page_size, cache_ttl_seconds).await
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

pub async fn get_books_by_rating(
    pool: &PgPool,
    min_rating: f64,
    page_num: i32,
    page_size: i32,
) -> Result<Vec<Rating>, sqlx::Error> {
    let offset = (page_num - 1) * page_size;
    let ratings: Vec<Rating> = sqlx::query_as::<_, Rating>(BOOKS_BY_RATING_QUERY)
        .bind(min_rating)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(ratings)
}

pub async fn get_books_by_popularity(
    pool: &PgPool,
    min_num_ratings: i32,
    page_num: i32,
    page_size: i32,
) -> Result<Vec<Rating>, sqlx::Error> {
    let offset = (page_num - 1) * page_size;
    let ratings: Vec<Rating> = sqlx::query_as::<_, Rating>(BOOKS_BY_POPULARITY_QUERY)
        .bind(min_num_ratings)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(ratings)
}
