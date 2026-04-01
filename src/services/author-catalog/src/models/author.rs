#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Author {
    pub name: String,
    pub id: i32,
}