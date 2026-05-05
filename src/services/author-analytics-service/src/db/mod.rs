use sqlx::PgPool;
use crate::models::{AuthorBookRow, RankedAuthorRow};

#[derive(Clone)]
pub struct AuthorAnalyticsDb {
    book_pool:   PgPool,
    rating_pool: PgPool,
}

// Internal flat row fetched from book-db
#[derive(sqlx::FromRow)]
struct BookRow {
    isbn:     i64,
    name:     String,
    pub_year: Option<i32>,
}

// Internal flat row fetched from rating-db
#[derive(sqlx::FromRow)]
struct RatingRow {
    book_isbn:   i64,
    star_rating: Option<f64>,
    num_ratings: Option<i64>,
}

impl AuthorAnalyticsDb {
    pub fn new(book_pool: PgPool, rating_pool: PgPool) -> Self {
        Self { book_pool, rating_pool }
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    async fn fetch_all_books(&self) -> Result<Vec<BookRow>, sqlx::Error> {
        sqlx::query_as::<_, BookRow>(
            "SELECT isbn, name, pub_year FROM book ORDER BY isbn LIMIT 1000",
        )
        .fetch_all(&self.book_pool)
        .await
    }

    async fn fetch_all_ratings(&self) -> Result<Vec<RatingRow>, sqlx::Error> {
        sqlx::query_as::<_, RatingRow>(
            "SELECT book_isbn, star_rating, num_ratings FROM rating",
        )
        .fetch_all(&self.rating_pool)
        .await
    }

    // Join books + ratings in memory; returns only books that have a rating
    fn join_books_ratings(
        books: Vec<BookRow>,
        ratings: Vec<RatingRow>,
    ) -> Vec<AuthorBookRow> {
        use std::collections::HashMap;
        let rating_map: HashMap<i64, RatingRow> =
            ratings.into_iter().map(|r| (r.book_isbn, r)).collect();

        books
            .into_iter()
            .filter_map(|b| {
                let r = rating_map.get(&b.isbn)?;
                Some(AuthorBookRow {
                    author_id:   b.isbn as i32,   // isbn proxies as "author id"
                    author_name: b.name.clone(),
                    book_isbn:   b.isbn,
                    book_name:   b.name,
                    pub_year:    b.pub_year.unwrap_or(0),
                    star_rating: r.star_rating.unwrap_or(0.0),
                    num_ratings: r.num_ratings.unwrap_or(0),
                    genre_name:  None,
                })
            })
            .collect()
    }

    // ── public query methods (same signatures as before) ─────────────────────

    pub async fn rank_authors_by_avg_rating(
        &self,
    ) -> Result<Vec<RankedAuthorRow>, sqlx::Error> {
        let books   = self.fetch_all_books().await?;
        let ratings = self.fetch_all_ratings().await?;
        let mut rows = Self::join_books_ratings(books, ratings);
        rows.sort_by(|a, b| {
            b.star_rating.partial_cmp(&a.star_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(rows
            .into_iter()
            .map(|r| RankedAuthorRow {
                author_id:     r.author_id,
                author_name:   r.author_name,
                average_rating: r.star_rating,
                total_ratings:  r.num_ratings,
            })
            .collect())
    }

    pub async fn rank_authors_by_total_ratings(
        &self,
    ) -> Result<Vec<RankedAuthorRow>, sqlx::Error> {
        let books   = self.fetch_all_books().await?;
        let ratings = self.fetch_all_ratings().await?;
        let mut rows = Self::join_books_ratings(books, ratings);
        rows.sort_by(|a, b| b.num_ratings.cmp(&a.num_ratings));
        Ok(rows
            .into_iter()
            .map(|r| RankedAuthorRow {
                author_id:     r.author_id,
                author_name:   r.author_name,
                average_rating: r.star_rating,
                total_ratings:  r.num_ratings,
            })
            .collect())
    }

    pub async fn author_books_with_ratings(
        &self,
        author_name: Option<&str>,
        author_id: Option<i32>,
        pub_year_from: Option<i32>,
        pub_year_to: Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        let books   = self.fetch_all_books().await?;
        let ratings = self.fetch_all_ratings().await?;
        let mut rows = Self::join_books_ratings(books, ratings);

        // filter by name (ILIKE simulation)
        if let Some(name) = author_name {
            let lower = name.to_lowercase();
            rows.retain(|r| r.author_name.to_lowercase().contains(&lower));
        }
        // filter by isbn-as-id
        if let Some(id) = author_id {
            rows.retain(|r| r.author_id == id);
        }
        if let Some(from) = pub_year_from {
            rows.retain(|r| r.pub_year >= from);
        }
        if let Some(to) = pub_year_to {
            rows.retain(|r| r.pub_year <= to);
        }

        rows.sort_by_key(|r| r.pub_year);
        Ok(rows)
    }

    pub async fn author_rating_per_book(
        &self,
        author_name: Option<&str>,
        author_id: Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        self.author_books_with_ratings(author_name, author_id, None, None)
            .await
    }

    pub async fn author_books_chronological(
        &self,
        author_name: Option<&str>,
        author_id: Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        self.author_books_with_ratings(author_name, author_id, None, None)
            .await
    }
}