use sqlx::PgPool;

use crate::models::rating::Rating;

const DEFAULT_PAGE_SIZE: i64 = 10;
const DEFAULT_PAGE_NUMBER: i64 = 1;

pub async fn get_ratings_query(
    pool: &PgPool,
    page_number: Option<i64>,
    page_size: Option<i64>,
) -> Result<Vec<Rating>, sqlx::Error> {
    let (limit, offset) = normalize_pagination(page_number, page_size);

    let ratings = sqlx::query_as::<_, Rating>(
        r#"
        SELECT
            book_isbn,
            star_rating,
            num_ratings
        FROM rating
        ORDER BY book_isbn
        LIMIT $1
        OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(ratings)
}

pub async fn get_rating_by_id_query(
    pool: &PgPool,
    book_isbn: i64,
) -> Result<Option<Rating>, sqlx::Error> {
    let rating = sqlx::query_as::<_, Rating>(
        r#"
        SELECT
            book_isbn,
            star_rating,
            num_ratings
        FROM rating
        WHERE book_isbn = $1
        "#,
    )
    .bind(book_isbn)
    .fetch_optional(pool)
    .await?;

    Ok(rating)
}

pub async fn add_rating_query(
    pool: &PgPool,
    book_isbn: i64,
    num_ratings: i64,
    star_rating: f64,
) -> Result<Rating, sqlx::Error> {
    let rating = sqlx::query_as::<_, Rating>(
        r#"
        INSERT INTO rating (
            book_isbn,
            star_rating,
            num_ratings
        )
        VALUES ($1, $2, $3)
        RETURNING
            book_isbn,
            star_rating,
            num_ratings
        "#,
    )
    .bind(book_isbn)
    .bind(star_rating)
    .bind(num_ratings)
    .fetch_one(pool)
    .await?;

    Ok(rating)
}

pub async fn update_rating_query(
    pool: &PgPool,
    book_isbn: i64,
    num_ratings: i64,
    star_rating: f64,
) -> Result<Option<Rating>, sqlx::Error> {
    let rating = sqlx::query_as::<_, Rating>(
        r#"
        UPDATE rating
        SET
            star_rating = $2,
            num_ratings = $3
        WHERE book_isbn = $1
        RETURNING
            book_isbn,
            star_rating,
            num_ratings
        "#,
    )
    .bind(book_isbn)
    .bind(star_rating)
    .bind(num_ratings)
    .fetch_optional(pool)
    .await?;

    Ok(rating)
}

pub async fn delete_rating_query(pool: &PgPool, book_isbn: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM rating
        WHERE book_isbn = $1
        "#,
    )
    .bind(book_isbn)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

fn normalize_pagination(page_number: Option<i64>, page_size: Option<i64>) -> (i64, i64) {
    let page_number = page_number.unwrap_or(DEFAULT_PAGE_NUMBER).max(1);

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).max(1);
    let offset = (page_number - 1) * page_size;

    (page_size, offset)
}
