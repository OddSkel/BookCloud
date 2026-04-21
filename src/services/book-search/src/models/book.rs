#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Book {
    pub isbn: i64,
    pub name: String,
    pub url: String,
    pub year_published: i32,
}