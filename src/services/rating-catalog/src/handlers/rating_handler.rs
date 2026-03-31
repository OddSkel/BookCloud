use sqlx::{PgPool};

use crate::db::rating::{get_rating_by_id_query, get_ratings_query};
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
