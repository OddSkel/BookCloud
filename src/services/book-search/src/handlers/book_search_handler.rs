use tonic::Request;

use crate::grpc::contracts::author_catalog::{
    GetAuthorsByNameRequest, author_catalog_grpc_client::AuthorCatalogGrpcClient,
};
use crate::grpc::contracts::book_catalog::{
    Book as CatalogBook, GetBooksRequest, book_catalog_grpc_client::BookCatalogGrpcClient,
};
use crate::grpc::contracts::book_search::{Book as ProtoBook, BookSearchResponse, Query};

const MAX_SEARCH_RESULTS: usize = 50;

pub async fn book_search(
    book_catalog_grpc_url: &str,
    author_catalog_grpc_url: &str,
    search_page_size: i32,
    search_max_pages: i32,
    params: Query,
) -> Result<BookSearchResponse, String> {
    let author_id = get_author_id(author_catalog_grpc_url, params.author.as_deref()).await?;

    let mut book_client = BookCatalogGrpcClient::connect(book_catalog_grpc_url.to_string())
        .await
        .map_err(|e| e.to_string())?;

    let page_size = search_page_size.max(1);
    let max_pages = search_max_pages.max(1);

    let mut books = Vec::new();

    for page_num in 1..=max_pages {
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

        if response.books.is_empty() {
            break;
        }

        for book in response.books {
            if matches_query(&book, &params) {
                books.push(to_proto_book(book));

                if books.len() >= MAX_SEARCH_RESULTS {
                    return Ok(BookSearchResponse { books });
                }
            }
        }
    }

    Ok(BookSearchResponse { books })
}

async fn get_author_id(
    author_catalog_grpc_url: &str,
    author_name: Option<&str>,
) -> Result<Option<i64>, String> {
    let Some(author_name) = author_name else {
        return Ok(None);
    };

    let author_name = author_name.trim();

    if author_name.is_empty() {
        return Ok(None);
    }

    let channel = tonic::transport::Channel::from_shared(author_catalog_grpc_url.to_string())
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
        .find(|author| contains_case_insensitive(&author.name, author_name));

    Ok(matched.map(|author| author.author_id))
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

fn to_proto_book(book: CatalogBook) -> ProtoBook {
    ProtoBook {
        isbn: book.isbn,
        name: book.name,
        url: book.url,
        pub_year: book.pub_year,
    }
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value.to_lowercase().contains(&needle.to_lowercase())
}