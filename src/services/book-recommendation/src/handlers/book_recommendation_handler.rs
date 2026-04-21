use sqlx::{PgPool, Arguments};
use sqlx::postgres::PgArguments;
use crate::models::book::Book as Model_Book;
// Ensure these imports match your specific recommendation proto contract
use crate::grpc::contracts::book_recommendation::{Query, BookRecommendationResponse, Book as Proto_Book};

pub async fn book_recommendation(
    pool: &PgPool,
    params: Query
) -> Result<BookRecommendationResponse, String> {

    let mut conditions = Vec::new();
    let mut args = PgArguments::default();
    let mut i = 1;

    if let Some(genre) = params.genre.as_deref().filter(|s| !s.trim().is_empty()) {
        conditions.push(format!("genre ILIKE ${i}"));
        args.add(format!("%{}%", genre)).map_err(|e| e.to_string())?;
        i += 1;
    }

    if let Some(rating) = params.rating {
            conditions.push(format!("rating >= ${i}"));
            args.add(rating).map_err(|e| e.to_string())?;
            i += 1;
        }

    if let Some(popularity) = params.popularity {
        // We use >= for a numeric popularity recommendation
        conditions.push(format!("popularity >= ${i}"));
        args.add(popularity).map_err(|e| e.to_string())?;
    }

    // Building the WHERE clause
    let where_clause = if conditions.is_empty() {
        "TRUE".to_string()
    } else {
        // Using "OR" as per your logic to broaden recommendations
        conditions.join(" OR ")
    };

    let query = format!("SELECT * FROM books WHERE {}", where_clause);

    let books_recommended = sqlx::query_as_with::<_, Model_Book, _>(&query, args)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(BookRecommendationResponse {
        books: books_recommended
            .into_iter()
            .map(|b| Proto_Book {
                isbn: b.isbn,
                name: b.name,
                url: b.url,
                pub_year: b.year_published,
            })
            .collect(),
    })
}