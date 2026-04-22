use tonic::Request;

use crate::grpc::contracts::book_catalog::{
    book_catalog_grpc_client::BookCatalogGrpcClient, Book as CatalogBook, GetBooksRequest,
};
use crate::grpc::contracts::book_search::{Query, BookSearchResponse, Book as Proto_Book};

pub async fn book_search(
    book_catalog_grpc_url: &str,
    search_page_size: i32,
    search_max_pages: i32,
    params: Query,
) -> Result<BookSearchResponse, String> {
    if params.author.as_deref().is_some_and(|author| !author.trim().is_empty()) {
        return Err("author filtering is not available in book-catalog responses".to_string());
    }

    let mut client = BookCatalogGrpcClient::connect(book_catalog_grpc_url.to_string())
        .await
        .map_err(|e| e.to_string())?;

    let page_size = search_page_size.max(1);
    let max_pages = search_max_pages.max(1);
    let mut page_num = 1;
    let mut books = Vec::new();

    loop {
        let response = client
            .get_books(Request::new(GetBooksRequest {
                page_num,
                page_size,
            }))
            .await
            .map(|r| r.into_inner())
            .map_err(|e| e.to_string())?;

        for book in response.books {
            if matches_query(&book, &params) {
                books.push(Proto_Book {
                    isbn: book.isbn,
                    name: book.name,
                    url: book.url,
                    pub_year: book.pub_year,
                });
            }
        }

        if page_num >= response.total_pages || page_num >= max_pages || response.total_pages == 0 {
            break;
        }

        page_num += 1;
    }

    Ok(BookSearchResponse { books })
}

fn matches_query(book: &CatalogBook, params: &Query) -> bool {
    let title_matches = params
        .title
        .as_deref()
        .is_none_or(|title| contains_case_insensitive(&book.name, title));

    let keywords_matches = params
        .keywords
        .as_deref()
        .is_none_or(|keywords| contains_case_insensitive(&book.summary_clean, keywords));

    title_matches && keywords_matches
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value.to_lowercase().contains(&needle.to_lowercase())
}
