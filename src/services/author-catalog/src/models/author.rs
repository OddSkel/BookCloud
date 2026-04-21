#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Author {
    pub name: String,
    pub author_id: i64,
}
