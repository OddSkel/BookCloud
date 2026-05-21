use sqlx::PgPool;
use crate::models::{AuthorBookRow, RankedAuthorRow};

#[derive(Clone)]
pub struct AuthorAnalyticsDb {
    book_pool:   PgPool,
    rating_pool: PgPool,
    author_pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct BookRow {
    isbn:     i64,
    pub_year: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct RatingRow {
    book_isbn:   i64,
    star_rating: Option<f64>,
    num_ratings: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct AuthorRow {
    author_id:   i64,
    author_name: String,
    book_isbn:   i64,
}

impl AuthorAnalyticsDb {
    pub fn new(book_pool: PgPool, rating_pool: PgPool, author_pool: PgPool) -> Self {
        Self { book_pool, rating_pool, author_pool }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    async fn fetch_all_books(&self) -> Result<Vec<BookRow>, sqlx::Error> {
        sqlx::query_as::<_, BookRow>(
            "SELECT isbn, pub_year FROM book ORDER BY isbn LIMIT 5000",
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

    async fn fetch_all_author_books(&self) -> Result<Vec<AuthorRow>, sqlx::Error> {
        sqlx::query_as::<_, AuthorRow>(
            r#"
            SELECT ba.author_id, a.name AS author_name, ba.book_isbn
            FROM book_author ba
            JOIN author a ON a.author_id = ba.author_id
            "#,
        )
        .fetch_all(&self.author_pool)
        .await
    }

    async fn full_join(
        &self,
        author_name:   Option<&str>,
        author_id:     Option<i32>,
        pub_year_from: Option<i32>,
        pub_year_to:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        use std::collections::HashMap;

        let books        = self.fetch_all_books().await?;
        let ratings      = self.fetch_all_ratings().await?;
        let author_books = self.fetch_all_author_books().await?;

        let book_map: HashMap<i64, &BookRow> =
            books.iter().map(|b| (b.isbn, b)).collect();
        let rating_map: HashMap<i64, &RatingRow> =
            ratings.iter().map(|r| (r.book_isbn, r)).collect();

        let mut rows: Vec<AuthorBookRow> = author_books
            .iter()
            .filter_map(|ab| {
                let b = book_map.get(&ab.book_isbn)?;
                let r = rating_map.get(&ab.book_isbn)?;
                Some(AuthorBookRow {
                    author_id:   ab.author_id as i32,
                    author_name: ab.author_name.clone(),
                    book_isbn:   ab.book_isbn,
                    book_name:   String::new(),
                    pub_year:    b.pub_year.unwrap_or(0),
                    star_rating: r.star_rating.unwrap_or(0.0),
                    num_ratings: r.num_ratings.unwrap_or(0),
                    genre_name:  None,
                })
            })
            .collect();

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
        rows.sort_by_key(|b| std::cmp::Reverse(b.num_ratings));
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

    // ── public query methods ──────────────────────────────────────────────────

    pub async fn rank_authors_by_avg_rating(
        &self,
    ) -> Result<Vec<RankedAuthorRow>, sqlx::Error> {
        use std::collections::HashMap;
        let rows = self.full_join(None, None, None, None).await?;

        // accumulator: (rating_sum, num_ratings_sum, book_count)
        let mut per_author: HashMap<(i32, String), (f64, i64, i32)> = HashMap::new();
        for r in rows {
            let e = per_author
                .entry((r.author_id, r.author_name))
                .or_insert((0.0, 0i64, 0i32));
            e.0 += r.star_rating;
            e.1 += r.num_ratings;
            e.2 += 1;
        }

        let mut results: Vec<RankedAuthorRow> = per_author
            .into_iter()
            .map(|((id, name), (sum, total_ratings, count))| RankedAuthorRow {
                author_id:      id,
                author_name:    name,
                average_rating: sum / count as f64,
                total_ratings,
            })
            .collect();

        results.sort_by(|a, b| b.average_rating.partial_cmp(&a.average_rating)
            .unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    pub async fn rank_authors_by_total_ratings(
        &self,
    ) -> Result<Vec<RankedAuthorRow>, sqlx::Error> {
        use std::collections::HashMap;
        let rows = self.full_join(None, None, None, None).await?;

        let mut per_author: HashMap<(i32, String), (f64, i64, i32)> = HashMap::new();
        for r in rows {
            let e = per_author
                .entry((r.author_id, r.author_name))
                .or_insert((0.0, 0i64, 0i32));
            e.0 += r.star_rating;
            e.1 += r.num_ratings;
            e.2 += 1;
        }

        let mut results: Vec<RankedAuthorRow> = per_author
            .into_iter()
            .map(|((id, name), (sum, total_ratings, count))| RankedAuthorRow {
                author_id:      id,
                author_name:    name,
                average_rating: sum / count as f64,
                total_ratings,
            })
            .collect();

        results.sort_by(|a, b| b.total_ratings.cmp(&a.total_ratings));
        Ok(results)
    }

    pub async fn author_books_with_ratings(
        &self,
        author_name:   Option<&str>,
        author_id:     Option<i32>,
        pub_year_from: Option<i32>,
        pub_year_to:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        self.full_join(author_name, author_id, pub_year_from, pub_year_to).await
    }

    pub async fn author_rating_per_book(
        &self,
        author_name: Option<&str>,
        author_id:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        self.full_join(author_name, author_id, None, None).await
    }

    pub async fn author_books_chronological(
        &self,
        author_name: Option<&str>,
        author_id:   Option<i32>,
    ) -> Result<Vec<AuthorBookRow>, sqlx::Error> {
        self.full_join(author_name, author_id, None, None).await
    }
}
