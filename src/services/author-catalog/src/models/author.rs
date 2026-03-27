use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Author {
    pub name: String,
    pub gender: String,
    pub year_born: i32,
    pub books_published: i32,
    pub id: i32,
}