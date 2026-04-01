use sqlx::{PgPool};
use sqlx::Arguments;
use sqlx::postgres::PgArguments;
use crate::models::book::Book as Model_Book;
use crate::grpc::contracts::book_recommendation::{Query, BookRecommendationResponse, Book as Proto_Book};

pub async fn book_recommendation(pool: &PgPool, params: Query) -> Result<BookRecommendationResponse, sqlx:: Error> {

    let mut conditions = Vec::new();
    let mut args = PgArguments::default();
    let mut i = 1;

    if let Some(genre) = &params.genre {
        conditions.push(format!("genre ILIKE ${i}"));
        let _ = args.add(format!("%{}%", genre));
        i += 1;
    }

    if let Some(rating) = &params.rating {
        conditions.push(format!("rating ILIKE ${i}"));
        let _ = args.add(format!("%{}%", rating));
        i += 1;
    }

    if let Some(popularity) = &params.popularity {
        conditions.push(format!("popularity ILIKE ${i}"));
        let _ = args.add(format!("%{}%", popularity));
    }

    let where_clause = if conditions.is_empty() {
        "TRUE".to_string()
    } else {
        conditions.join(" OR ")
    };

    let query = format!("SELECT * FROM books WHERE {}", where_clause);

    let books_recommendated = sqlx::query_as_with::<_,Model_Book,_>(&query, args)
    .fetch_all(pool)
    .await?;

    Ok(BookRecommendationResponse{
        books : books_recommendated.into_iter().map(
            |b| Proto_Book {
                id: b.id,
                name: b.name,
                author: b.author,
                isbn: b.isbn,
                year_published: b.year_published,
                editor: b.editor,
                edition_number: b.edition_number,
                genre: b.genre,
                summary: b.summary,
            }
        ).collect(),
    })
}
