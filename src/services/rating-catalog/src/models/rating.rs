use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rating {
    pub book_isbn: i64,
    pub star_rating: f64,
    pub num_ratings: i64,
}
