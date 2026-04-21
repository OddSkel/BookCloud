use sqlx::{PgPool};
use sqlx::Arguments;
use sqlx::postgres::PgArguments;
use crate::models::book::Book as Model_Book;
use crate::grpc::contracts::book_search::{Query, BookSearchResponse, Book as Proto_Book};

pub async fn book_search(pool: &PgPool, params: Query) -> Result<BookSearchResponse, sqlx:: Error> {

    let mut conditions = Vec::new();
    let mut args = PgArguments::default();
    let mut i = 1;

    if let Some(title) = &params.title {
        conditions.push(format!("title ILIKE ${i}"));
        let _ = args.add(format!("%{}%", title));
        i += 1;
    }

    if let Some(author) = &params.author {
        conditions.push(format!("author ILIKE ${i}"));
        let _ = args.add(format!("%{}%", author));
        i += 1;
    }

    if let Some(keywords) = &params.keywords {
        conditions.push(format!("summary ILIKE ${i}"));
        let _ = args.add(format!("%{}%", keywords));
    }

    let where_clause = if conditions.is_empty() {
        "TRUE".to_string()
    } else {
        conditions.join(" OR ")
    };

    let query = format!("SELECT * FROM books WHERE {}", where_clause);

    let books_searched = sqlx::query_as_with::<_,Model_Book,_>(&query, args)
    .fetch_all(pool)
    .await?;

    Ok(BookSearchResponse{
        books : books_searched.into_iter().map(
            |b| Proto_Book {
                isbn: b.isbn,
                name: b.name,
                url: b.url,
                pub_year: b.year_published
            }
        ).collect(),
    })
}
