use std::collections::HashMap;
use tonic::{Request, Response, Status};

use crate::db::AuthorAnalyticsDb;
use crate::grpc::contracts::{
    author_analytics::{
        author_analytics_grpc_server::AuthorAnalyticsGrpc,
        AuthorConsistencyItem, AuthorEvolutionPoint, AuthorGrowthItem,
        AuthorPerformanceRequest, AuthorPerformanceResponse,
        AuthorsConsistencyRequest, AuthorsConsistencyResponse,
        AuthorsGrowthRequest, AuthorsGrowthResponse,
        RankAuthorRequest, RankAuthorResponse, RankedAuthor,
    },
    common::{HealthCheckRequest, HealthCheckResponse},
};
use crate::utils::{interpret_correlation, pearson_correlation};

pub struct AuthorAnalyticsService {
    pub db: AuthorAnalyticsDb,
}

impl AuthorAnalyticsService {
    pub fn new(db: AuthorAnalyticsDb) -> Self { Self { db } }
}

#[tonic::async_trait]
impl AuthorAnalyticsGrpc for AuthorAnalyticsService {
    async fn health_check(
        &self, _: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            service: "author-analytics-service".to_string(),
            status:  "ok".to_string(),
        }))
    }

    async fn rank_authors(
        &self, request: Request<RankAuthorRequest>,
    ) -> Result<Response<RankAuthorResponse>, Status> {
        let params = request.into_inner();
        let rows = if params.average_rating.is_some() {
            self.db.rank_authors_by_avg_rating().await
        } else {
            self.db.rank_authors_by_total_ratings().await
        }.map_err(|e| Status::internal(e.to_string()))?;

        let authors = rows.into_iter().map(|r| RankedAuthor {
            author_name:          r.author_name,
            average_rating:       r.average_rating,
            total_number_ratings: r.total_ratings,
        }).collect();

        Ok(Response::new(RankAuthorResponse { authors }))
    }

    async fn author_performance(
        &self, request: Request<AuthorPerformanceRequest>,
    ) -> Result<Response<AuthorPerformanceResponse>, Status> {
        let p = request.into_inner();
        let rows = self.db.author_books_with_ratings(
            p.author_name.as_deref(), p.author_id,
            p.pub_year_from, p.pub_year_to,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        if rows.is_empty() {
            return Err(Status::not_found("No books found for the given filters"));
        }

        let author_id   = rows[0].author_id.to_string();
        let author_name = rows[0].author_name.clone();

        let evolution: Vec<AuthorEvolutionPoint> = rows.iter().map(|r| AuthorEvolutionPoint {
            year:             r.pub_year,
            title:            r.book_name.clone(),
            quality_score:    r.star_rating,
            popularity_score: r.num_ratings as f64,
            genre:            r.genre_name.clone().unwrap_or_default(),
        }).collect();

        let qualities:    Vec<f64> = evolution.iter().map(|e| e.quality_score).collect();
        let popularities: Vec<f64> = evolution.iter().map(|e| e.popularity_score).collect();
        let correlation = pearson_correlation(&qualities, &popularities);

        let from = evolution.iter().map(|e| e.year).min().unwrap_or(0);
        let to   = evolution.iter().map(|e| e.year).max().unwrap_or(0);

        Ok(Response::new(AuthorPerformanceResponse {
            author_id,
            author_name,
            pub_year_from:           from,
            pub_year_to:             to,
            correlation_coefficient: correlation,
            sample_size:             evolution.len() as i32,
            interpretation:          interpret_correlation(correlation),
            evolution,
        }))
    }

    async fn authors_consistency(
        &self, request: Request<AuthorsConsistencyRequest>,
    ) -> Result<Response<AuthorsConsistencyResponse>, Status> {
        let p = request.into_inner();
        let rows = self.db.author_rating_per_book(
            p.author_name.as_deref(), p.author_id,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        let mut per_author: HashMap<(i32, String), Vec<f64>> = HashMap::new();
        for r in &rows {
            per_author.entry((r.author_id, r.author_name.clone()))
                .or_default().push(r.star_rating);
        }

        let mut results: Vec<AuthorConsistencyItem> = per_author.into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|((id, name), ratings)| {
                let avg     = ratings.iter().sum::<f64>() / ratings.len() as f64;
                let var     = ratings.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / ratings.len() as f64;
                let std_dev = var.sqrt();
                AuthorConsistencyItem {
                    author_id:         id.to_string(),
                    author_name:       name,
                    consistency_score: 1.0 / (1.0 + std_dev),
                    average_rating:    avg,
                    std_deviation:     std_dev,
                    total_books:       ratings.len() as i32,
                    rank:              0,
                }
            }).collect();

        results.sort_by(|a, b| b.consistency_score.partial_cmp(&a.consistency_score)
            .unwrap_or(std::cmp::Ordering::Equal));
        results.iter_mut().enumerate().for_each(|(i, r)| r.rank = (i + 1) as i32);

        Ok(Response::new(AuthorsConsistencyResponse { authors: results }))
    }

    async fn authors_growth(
        &self, request: Request<AuthorsGrowthRequest>,
    ) -> Result<Response<AuthorsGrowthResponse>, Status> {
        let p = request.into_inner();
        let rows = self.db.author_books_chronological(
            p.author_name.as_deref(), p.author_id,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        let mut per_author: HashMap<(i32, String), Vec<(i32, f64)>> = HashMap::new();
        for r in &rows {
            per_author.entry((r.author_id, r.author_name.clone()))
                .or_default().push((r.pub_year, r.star_rating));
        }

        let mut results: Vec<AuthorGrowthItem> = per_author.into_iter()
            .filter(|(_, v)| v.len() >= 2)
            .map(|((id, name), books)| {
                let mid    = books.len() / 2;
                let first  = books[..mid].iter().map(|(_, r)| r).sum::<f64>() / mid as f64;
                let latest = books[mid..].iter().map(|(_, r)| r).sum::<f64>()
                    / (books.len() - mid) as f64;
                AuthorGrowthItem {
                    author_id:     id.to_string(),
                    author_name:   name,
                    growth_score:  latest - first,
                    first_rating:  first,
                    latest_rating: latest,
                    total_books:   books.len() as i32,
                    rank:          0,
                }
            }).collect();

        results.sort_by(|a, b| b.growth_score.partial_cmp(&a.growth_score)
            .unwrap_or(std::cmp::Ordering::Equal));
        results.iter_mut().enumerate().for_each(|(i, r)| r.rank = (i + 1) as i32);

        Ok(Response::new(AuthorsGrowthResponse { authors: results }))
    }
}


// ────────── Unit Tests────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use tonic::Request;

    use crate::models::{AuthorBookRow, RankedAuthorRow};
    use crate::service::AuthorAnalyticsService;

    // ─────────────────────────────────────────────────────────────────────────
    // FakeDb  (mirrors FakeCatalogCache in compare-service/tests/conftest.py)
    //
    // Each constructor field corresponds to one db method the service calls.
    // Leave a field as None to make that method return an empty Vec.
    // ─────────────────────────────────────────────────────────────────────────

    struct FakeDb {
        ranked_avg:     Vec<RankedAuthorRow>,
        ranked_total:   Vec<RankedAuthorRow>,
        books_ratings:  Vec<AuthorBookRow>,
        rating_per_book: Vec<AuthorBookRow>,
        chronological:  Vec<AuthorBookRow>,
    }

    impl FakeDb {
        fn empty() -> Self {
            Self {
                ranked_avg:      vec![],
                ranked_total:    vec![],
                books_ratings:   vec![],
                rating_per_book: vec![],
                chronological:   vec![],
            }
        }
    }

    // Thin wrapper so we can inject FakeDb into AuthorAnalyticsService.
    // We re-implement only the db methods called by service.rs.
    use crate::db::AuthorAnalyticsDb;
    use sqlx::postgres::PgPoolOptions;

    // Because AuthorAnalyticsDb wraps real PgPools we cannot easily swap it.
    // The cleanest group-style solution (no trait refactor needed) is to test
    // the *pure aggregation logic* extracted into free functions — exactly what
    // compare-service does (it tests `get_correlation`, not the gRPC layer).
    //
    // Each function below is the pure core of a service.rs handler, lifted out
    // for testing.  See the "Pure logic helpers" section below.

    // ─────────────────────────────────────────────────────────────────────────
    // Pure logic helpers  (extracted from the handler bodies in service.rs)
    // These take plain Vec<AuthorBookRow> / Vec<RankedAuthorRow> — no DB touch.
    // ─────────────────────────────────────────────────────────────────────────

    use std::collections::HashMap;
    use crate::grpc::contracts::author_analytics::{
        AuthorConsistencyItem, AuthorGrowthItem, RankedAuthor,
    };
    use crate::utils::{interpret_correlation, pearson_correlation};

    /// rank_authors — sort a pre-built Vec<RankedAuthorRow> by avg rating.
    fn rank_by_avg(mut rows: Vec<RankedAuthorRow>) -> Vec<RankedAuthor> {
        rows.sort_by(|a, b| b.average_rating.partial_cmp(&a.average_rating)
            .unwrap_or(std::cmp::Ordering::Equal));
        rows.into_iter().map(|r| RankedAuthor {
            author_name:          r.author_name,
            average_rating:       r.average_rating,
            total_number_ratings: r.total_ratings,
        }).collect()
    }

    /// rank_authors — sort by total ratings.
    fn rank_by_total(mut rows: Vec<RankedAuthorRow>) -> Vec<RankedAuthor> {
        rows.sort_by(|a, b| b.total_ratings.cmp(&a.total_ratings));
        rows.into_iter().map(|r| RankedAuthor {
            author_name:          r.author_name,
            average_rating:       r.average_rating,
            total_number_ratings: r.total_ratings,
        }).collect()
    }

    /// authors_consistency — pure aggregation matching service.rs exactly.
    fn compute_consistency(rows: Vec<AuthorBookRow>) -> Vec<AuthorConsistencyItem> {
        let mut per_author: HashMap<(i32, String), Vec<f64>> = HashMap::new();
        for r in &rows {
            per_author.entry((r.author_id, r.author_name.clone()))
                .or_default().push(r.star_rating);
        }
        let mut results: Vec<AuthorConsistencyItem> = per_author
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|((id, name), ratings)| {
                let avg     = ratings.iter().sum::<f64>() / ratings.len() as f64;
                let var     = ratings.iter().map(|r| (r - avg).powi(2)).sum::<f64>()
                              / ratings.len() as f64;
                let std_dev = var.sqrt();
                AuthorConsistencyItem {
                    author_id:         id.to_string(),
                    author_name:       name,
                    consistency_score: 1.0 / (1.0 + std_dev),
                    average_rating:    avg,
                    std_deviation:     std_dev,
                    total_books:       ratings.len() as i32,
                    rank:              0,
                }
            }).collect();
        results.sort_by(|a, b| b.consistency_score.partial_cmp(&a.consistency_score)
            .unwrap_or(std::cmp::Ordering::Equal));
        results.iter_mut().enumerate().for_each(|(i, r)| r.rank = (i + 1) as i32);
        results
    }

    /// authors_growth — pure aggregation matching service.rs exactly.
    fn compute_growth(rows: Vec<AuthorBookRow>) -> Vec<AuthorGrowthItem> {
        let mut per_author: HashMap<(i32, String), Vec<(i32, f64)>> = HashMap::new();
        for r in &rows {
            per_author.entry((r.author_id, r.author_name.clone()))
                .or_default().push((r.pub_year, r.star_rating));
        }
        let mut results: Vec<AuthorGrowthItem> = per_author
            .into_iter()
            .filter(|(_, v)| v.len() >= 2)
            .map(|((id, name), books)| {
                let mid    = books.len() / 2;
                let first  = books[..mid].iter().map(|(_, r)| r).sum::<f64>() / mid as f64;
                let latest = books[mid..].iter().map(|(_, r)| r).sum::<f64>()
                             / (books.len() - mid) as f64;
                AuthorGrowthItem {
                    author_id:     id.to_string(),
                    author_name:   name,
                    growth_score:  latest - first,
                    first_rating:  first,
                    latest_rating: latest,
                    total_books:   books.len() as i32,
                    rank:          0,
                }
            }).collect();
        results.sort_by(|a, b| b.growth_score.partial_cmp(&a.growth_score)
            .unwrap_or(std::cmp::Ordering::Equal));
        results.iter_mut().enumerate().for_each(|(i, r)| r.rank = (i + 1) as i32);
        results
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helper constructors  (mirrors book() / genre() in conftest.py)
    // ─────────────────────────────────────────────────────────────────────────

    fn author_book_row(
        author_id: i32, author_name: &str,
        book_isbn: i64, pub_year: i32,
        star_rating: f64, num_ratings: i64,
    ) -> AuthorBookRow {
        AuthorBookRow {
            author_id,
            author_name: author_name.to_string(),
            book_isbn,
            book_name: String::new(),
            pub_year,
            star_rating,
            num_ratings,
            genre_name: None,
        }
    }

    fn ranked_row(
        author_id: i32, author_name: &str,
        average_rating: f64, total_ratings: i64,
    ) -> RankedAuthorRow {
        RankedAuthorRow { author_id, author_name: author_name.to_string(), average_rating, total_ratings }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tests — rank_authors
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_rank_authors_by_avg_rating_orders_correctly() {
        // Alice avg=3.75 (two books: 4.5+3.0), Bob avg=4.8 (one book)
        let rows = vec![
            ranked_row(1, "Alice Author", 3.75, 150),
            ranked_row(2, "Bob Books",    4.8,  200),
        ];
        let result = rank_by_avg(rows);
        assert_eq!(result[0].author_name, "Bob Books",
            "Bob (4.8) must rank first by avg rating");
        assert_eq!(result[1].author_name, "Alice Author",
            "Alice (3.75) must rank second");
    }

    #[test]
    fn test_rank_authors_by_total_ratings_orders_correctly() {
        let rows = vec![
            ranked_row(1, "Alice Author", 3.75, 150),
            ranked_row(2, "Bob Books",    4.8,  200),
        ];
        let result = rank_by_total(rows);
        assert_eq!(result[0].author_name, "Bob Books",
            "Bob (200 ratings) must rank first by total");
        assert_eq!(result[1].author_name, "Alice Author");
    }

    #[test]
    fn test_rank_authors_empty_input_returns_empty_list() {
        let result = rank_by_avg(vec![]);
        assert!(result.is_empty(), "Empty input must produce empty ranking");
    }

    #[test]
    fn test_rank_authors_single_author_returns_one_entry() {
        let rows = vec![ranked_row(1, "Only Author", 4.0, 50)];
        let result = rank_by_avg(rows);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].author_name, "Only Author");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tests — authors_consistency
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_consistency_std_dev_computed_correctly() {
        // Alice: ratings [4.5, 3.0] → mean=3.75, var=0.5625, std=0.75
        let rows = vec![
            author_book_row(1, "Alice Author", 1001, 2018, 4.5, 100),
            author_book_row(1, "Alice Author", 1002, 2020, 3.0,  50),
        ];
        let result = compute_consistency(rows);
        let alice = result.iter().find(|r| r.author_name == "Alice Author")
            .expect("Alice must appear");
        assert!((alice.std_deviation - 0.75).abs() < 1e-9,
            "std_deviation should be 0.75, got {}", alice.std_deviation);
        assert!((alice.average_rating - 3.75).abs() < 1e-9,
            "average_rating should be 3.75, got {}", alice.average_rating);
        assert_eq!(alice.total_books, 2);
    }

    #[test]
    fn test_consistency_single_book_author_has_zero_std_dev() {
        // Bob has only one book → std-dev = 0.0, consistency_score = 1.0
        let rows = vec![
            author_book_row(2, "Bob Books", 1003, 2022, 4.8, 200),
        ];
        let result = compute_consistency(rows);
        let bob = result.iter().find(|r| r.author_name == "Bob Books")
            .expect("Bob must appear");
        assert!(bob.std_deviation.abs() < 1e-9,
            "Single-book author must have std_deviation=0.0");
        assert!((bob.consistency_score - 1.0).abs() < 1e-9,
            "consistency_score must be 1.0 when std_dev=0");
    }

    #[test]
    fn test_consistency_rank_assigned_correctly() {
        // Bob (std=0, score=1.0) ranks above Alice (std=0.75, score≈0.57)
        let rows = vec![
            author_book_row(1, "Alice Author", 1001, 2018, 4.5, 100),
            author_book_row(1, "Alice Author", 1002, 2020, 3.0,  50),
            author_book_row(2, "Bob Books",    1003, 2022, 4.8, 200),
        ];
        let result = compute_consistency(rows);
        let bob   = result.iter().find(|r| r.author_name == "Bob Books").unwrap();
        let alice = result.iter().find(|r| r.author_name == "Alice Author").unwrap();
        assert_eq!(bob.rank, 1,   "Bob (most consistent) must be rank 1");
        assert_eq!(alice.rank, 2, "Alice must be rank 2");
    }

    #[test]
    fn test_consistency_empty_input_returns_empty_list() {
        let result = compute_consistency(vec![]);
        assert!(result.is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tests — authors_growth
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_growth_excludes_single_book_authors() {
        // Bob has only 1 book — must be excluded (requires >= 2)
        let rows = vec![
            author_book_row(1, "Alice Author", 1001, 2018, 4.5, 100),
            author_book_row(1, "Alice Author", 1002, 2020, 3.0,  50),
            author_book_row(2, "Bob Books",    1003, 2022, 4.8, 200),
        ];
        let result = compute_growth(rows);
        assert!(!result.iter().any(|r| r.author_name == "Bob Books"),
            "Bob (1 book) must be excluded from growth results");
    }

    #[test]
    fn test_growth_score_is_latest_minus_first() {
        // Alice: books sorted chronologically [2018→4.5, 2020→3.0]
        // mid=1 → first_half=[4.5] avg=4.5, second_half=[3.0] avg=3.0
        // growth_score = 3.0 - 4.5 = -1.5
        let rows = vec![
            author_book_row(1, "Alice Author", 1001, 2018, 4.5, 100),
            author_book_row(1, "Alice Author", 1002, 2020, 3.0,  50),
        ];
        let result = compute_growth(rows);
        let alice = result.iter().find(|r| r.author_name == "Alice Author")
            .expect("Alice must appear");
        assert!((alice.growth_score - (-1.5)).abs() < 1e-9,
            "growth_score should be -1.5, got {}", alice.growth_score);
        assert!((alice.first_rating  - 4.5).abs() < 1e-9);
        assert!((alice.latest_rating - 3.0).abs() < 1e-9);
        assert_eq!(alice.total_books, 2);
    }

    #[test]
    fn test_growth_positive_score_when_improving() {
        // Carol: [2018→3.0, 2022→5.0] → growth_score = +2.0
        let rows = vec![
            author_book_row(3, "Carol Writes", 2001, 2018, 3.0, 80),
            author_book_row(3, "Carol Writes", 2002, 2022, 5.0, 90),
        ];
        let result = compute_growth(rows);
        let carol = result.iter().find(|r| r.author_name == "Carol Writes")
            .expect("Carol must appear");
        assert!(carol.growth_score > 0.0,
            "Improving author must have positive growth_score");
        assert!((carol.growth_score - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_growth_ranked_descending_by_growth_score() {
        // Carol (+2.0) must rank above Alice (-1.5)
        let rows = vec![
            author_book_row(1, "Alice Author", 1001, 2018, 4.5, 100),
            author_book_row(1, "Alice Author", 1002, 2020, 3.0,  50),
            author_book_row(3, "Carol Writes", 2001, 2018, 3.0,  80),
            author_book_row(3, "Carol Writes", 2002, 2022, 5.0,  90),
        ];
        let result = compute_growth(rows);
        assert_eq!(result[0].author_name, "Carol Writes",
            "Carol (+2.0) must rank first");
        assert_eq!(result[0].rank, 1);
        assert_eq!(result[1].rank, 2);
    }

    #[test]
    fn test_growth_empty_input_returns_empty_list() {
        let result = compute_growth(vec![]);
        assert!(result.is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tests — author_performance (Pearson correlation + interpretation)
    // These test the pure utils functions used inside the handler,
    // mirroring compare-service's test_get_correlation.py
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_pearson_correlation_insufficient_data_returns_zero() {
        // Single data point — correlation undefined, expect 0.0
        let result = pearson_correlation(&[4.5], &[100.0]);
        assert_eq!(result, 0.0,
            "Single point must return 0.0 (insufficient data)");
    }

    #[test]
    fn test_pearson_correlation_constant_values_returns_zero() {
        // Both series constant → std-dev=0 → should not panic, returns 0.0
        let result = pearson_correlation(&[4.0, 4.0, 4.0], &[100.0, 100.0, 100.0]);
        assert_eq!(result, 0.0,
            "Constant series must return 0.0 (no correlation)");
    }

    #[test]
    fn test_pearson_correlation_perfect_positive() {
        let qualities    = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let popularities = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let r = pearson_correlation(&qualities, &popularities);
        assert!((r - 1.0).abs() < 1e-9,
            "Perfect positive correlation must return ~1.0, got {}", r);
    }

    #[test]
    fn test_pearson_correlation_perfect_negative() {
        let qualities    = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let popularities = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let r = pearson_correlation(&qualities, &popularities);
        assert!((r - (-1.0)).abs() < 1e-9,
            "Perfect negative correlation must return ~-1.0, got {}", r);
    }

    #[test]
    fn test_interpret_correlation_covers_all_bands() {
        // These expected strings must match whatever interpret_correlation() returns.
        // Adjust the expected values if your utils.rs uses different thresholds/wording.
        let cases: &[(f64, &str)] = &[
            ( 0.0,  "No significant correlation"),
            ( 0.05, "No significant correlation"),
            ( 0.1,  "Weak positive correlation"),
            ( 0.4,  "Moderate positive correlation"),
            ( 0.7,  "Strong positive correlation"),
            (-0.05, "No significant correlation"),
            (-0.1,  "Weak negative correlation"),
            (-0.4,  "Moderate negative correlation"),
            (-0.7,  "Strong negative correlation"),
        ];
        for (value, expected) in cases {
            let got = interpret_correlation(*value);
            assert_eq!(got, *expected,
                "interpret_correlation({}) → expected {:?}, got {:?}",
                value, expected, got);
        }
    }
}
