use sqlx::FromRow;

#[derive(FromRow)]
#[allow(dead_code)]
pub struct RankedAuthorRow {
    pub author_id: i32,
    pub author_name: String,
    pub average_rating: f64,
    pub total_ratings: i64,
}

#[derive(FromRow)]
#[allow(dead_code)]
pub struct AuthorBookRow {
    pub author_id: i32,
    pub author_name: String,
    pub book_isbn: i64,
    pub book_name: String,
    pub pub_year: i32,
    pub star_rating: f64,
    pub num_ratings: i64,
    pub genre_name: Option<String>,
}
