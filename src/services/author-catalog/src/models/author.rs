use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub author_id: i32,
}
