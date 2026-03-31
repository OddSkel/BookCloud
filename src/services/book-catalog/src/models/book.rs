#[derive(Clone)]
pub struct Book {
    pub id: String,
    pub name: String,
    pub author: String,
    pub genre: String,
    pub year_published: i32,
    pub isbn: String,
    pub summary: Option<String>,
}