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