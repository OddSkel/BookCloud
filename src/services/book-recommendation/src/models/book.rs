#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Book {
    pub id: i32,
    pub name: String,
    pub author: String,
    pub isbn: i32,
    pub year_published: i32,
    pub editor: String,
    pub edition_number: i32,
    pub genre: String,
    pub summary: Option<String>,
}