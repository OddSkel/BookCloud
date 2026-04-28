use tonic::Request;

use crate::grpc::contracts::author_catalog::{
    GetAuthorsByNameRequest, author_catalog_grpc_client::AuthorCatalogGrpcClient,
};
use crate::grpc::contracts::book_catalog::{
    Book as CatalogBook, GetBooksRequest, book_catalog_grpc_client::BookCatalogGrpcClient,
};
use crate::grpc::contracts::book_search::{Book as Proto_Book, BookSearchResponse, Query};

pub async fn book_search(
    book_catalog_grpc_url: &str,
    author_catalog_grpc_url: &str,
    search_page_size: i32,
    search_max_pages: i32,
    params: Query,
) -> Result<BookSearchResponse, String> {
    let author_id: Option<i32> = if let Some(author_name) = params.author.as_deref() {
        if author_name.trim().is_empty() {
            None
        } else {
            let channel =
                tonic::transport::Channel::from_shared(author_catalog_grpc_url.to_string())
                    .map_err(|e| e.to_string())?
                    .connect()
                    .await
                    .map_err(|e| e.to_string())?;

            let mut author_client =
                AuthorCatalogGrpcClient::new(channel).max_decoding_message_size(usize::MAX);

            let response = author_client
                .get_authors_by_name(Request::new(GetAuthorsByNameRequest {
                    name: author_name.to_string(),
                }))
                .await
                .map(|r| r.into_inner())
                .map_err(|e| e.to_string())?;

            let matched = response
                .authors
                .into_iter()
                .find(|a| contains_case_insensitive(&a.name, author_name));

            match matched {
                Some(a) => Some(a.author_id),
                None => return Ok(BookSearchResponse { books: vec![] }),
            }
        }
    } else {
        None
    };

    let mut book_client = BookCatalogGrpcClient::connect(book_catalog_grpc_url.to_string())
        .await
        .map_err(|e| e.to_string())?;

    let page_size = search_page_size.max(1);
    let max_pages = search_max_pages.max(1);
    let mut page_num = 1;
    let mut books = Vec::new();

    loop {
        let request = GetBooksRequest {
            page_num,
            page_size,
            author_id,
        };

        let response = book_client
            .get_books(Request::new(request))
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
