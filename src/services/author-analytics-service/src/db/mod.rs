use sqlx::PgPool;
use crate::models::{AuthorBookRow, RankedAuthorRow};

#[derive(Clone)]
pub struct AuthorAnalyticsDb {
    pool: PgPool,
}

impl AuthorAnalyticsDb {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn rank_authors_by_avg_rating(&self) -> Result<Vec<RankedAuthorRow>, sqlx::Error> {
        sqlx::query_as::<_, RankedAuthorRow>(
            r#"
            SELECT
                a.id                       AS author_id,
                a.name                     AS author_name,
                AVG(r.star_rating)::float8 AS average_rating,
                SUM(r.num_ratings)::int8   AS total_ratings
            FROM author a
            JOIN book_author ba ON a.id = ba.author_id
            JOIN book b         ON ba.book_isbn = b.isbn
            JOIN rating r       ON b.isbn::text = r.book_isbn
            GROUP BY a.id, a.name
            ORDER BY average_rating DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn rank_authors_by_total_ratings(&self) -> Result<Vec<RankedAuthorRow>, sqlx::Error> {
        sqlx::query_as::<_, RankedAuthorRow>(
            r#"
            SELECT
                a.id                       AS author_id,
                a.name                     AS author_name,
                AVG(r.star_rating)::float8 AS average_rating,
                SUM(r.num_ratings)::int8   AS total_ratings
            FROM author a
            JOIN book_author ba ON a.id = ba.author_id
            JOIN book b         ON ba.book_isbn = b.isbn
            JOIN rating r       ON b.isbn::text = r.book_isbn
            GROUP BY a.id, a.name
            ORDER BY total_ratings DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn author_books_with_ratings(
        &self,
        author_name:   Option<&str>,
        author_id:     Option<i32>,
        pub_year_from: Option<i32>,
        pub_year_to:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        sqlx::query_as::<_, AuthorBookRow>(
            r#"
            SELECT
                a.id                       AS author_id,
                a.name                     AS author_name,
                b.isbn                     AS book_isbn,
                b.name                     AS book_name,
                b.pub_year                 AS pub_year,
                r.star_rating::float8      AS star_rating,
                r.num_ratings::int8        AS num_ratings,
                MIN(g.name)                AS genre_name
            FROM author a
            JOIN book_author ba ON a.id = ba.author_id
            JOIN book b         ON ba.book_isbn = b.isbn
            JOIN rating r       ON b.isbn::text = r.book_isbn
            LEFT JOIN book_genre bg ON b.isbn = bg.book_isbn
            LEFT JOIN genre g       ON bg.genre_id = g.id
            WHERE ($1::text IS NULL OR a.name ILIKE '%' || $1 || '%')
              AND ($2::int  IS NULL OR a.id = $2)
              AND ($3::int  IS NULL OR b.pub_year >= $3)
              AND ($4::int  IS NULL OR b.pub_year <= $4)
            GROUP BY a.id, a.name, b.isbn, b.name, b.pub_year, r.star_rating, r.num_ratings
            ORDER BY b.pub_year ASC
            "#,
        )
        .bind(author_name)
        .bind(author_id)
        .bind(pub_year_from)
        .bind(pub_year_to)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn author_rating_per_book(
        &self,
        author_name: Option<&str>,
        author_id:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        sqlx::query_as::<_, AuthorBookRow>(
            r#"
            SELECT
                a.id                  AS author_id,
                a.name                AS author_name,
                b.isbn                AS book_isbn,
                b.name                AS book_name,
                b.pub_year            AS pub_year,
                r.star_rating::float8 AS star_rating,
                r.num_ratings::int8   AS num_ratings,
                NULL::text            AS genre_name
            FROM author a
            JOIN book_author ba ON a.id = ba.author_id
            JOIN book b         ON ba.book_isbn = b.isbn
            JOIN rating r       ON b.isbn::text = r.book_isbn
            WHERE ($1::text IS NULL OR a.name ILIKE '%' || $1 || '%')
              AND ($2::int  IS NULL OR a.id = $2)
            "#,
        )
        .bind(author_name)
        .bind(author_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn author_books_chronological(
        &self,
        author_name: Option<&str>,
        author_id:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        sqlx::query_as::<_, AuthorBookRow>(
            r#"
            SELECT
                a.id                  AS author_id,
                a.name                AS author_name,
                b.isbn                AS book_isbn,
                b.name                AS book_name,
                b.pub_year            AS pub_year,
                r.star_rating::float8 AS star_rating,
                r.num_ratings::int8   AS num_ratings,
                NULL::text            AS genre_name
            FROM author a
            JOIN book_author ba ON a.id = ba.author_id
            JOIN book b         ON ba.book_isbn = b.isbn
            JOIN rating r       ON b.isbn::text = r.book_isbn
            WHERE ($1::text IS NULL OR a.name ILIKE '%' || $1 || '%')
              AND ($2::int  IS NULL OR a.id = $2)
            ORDER BY b.pub_year ASC
            "#,
        )
        .bind(author_name)
        .bind(author_id)
        .fetch_all(&self.pool)
        .await
    }
}