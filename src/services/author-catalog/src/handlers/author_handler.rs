use crate::grpc::contracts::author_catalog::{
    AddAuthorResponse, Author as ProtoAuthor, AuthorDeleteResponse, DeleteAuthorRequest,
    GetAuthorRequest, GetAuthorResponse, GetAuthorsByNameRequest, GetAuthorsRequest,
    GetAuthorsResponse, UpdateAuthorRequest, UpdateAuthorResponse,
};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use anyhow::Result;
use crate::models::author::Author as Model_Author;
use sqlx::PgPool;

const DEFAULT_PAGE_SIZE: i64 = 10;
const DEFAULT_PAGE_NUMBER: i64 = 1;

pub async fn get_authors(
    pool: &PgPool,
    redis: &ConnectionManager,
    params: &GetAuthorsRequest,
    cache_ttl_seconds: u64,
) -> Result<GetAuthorsResponse> {
    let mut redis = redis.clone();
    let (pages, pages_size) = normalize_pagination(params.page, params.page_size);

    let authors_cache = format!("authors: page:{}:page_size:{}", pages, pages_size);

    if let Some(cache) = redis.get::<_, Option<String>>(authors_cache.clone()).await? {
        let cached_authors: Vec<ProtoAuthor> = serde_json::from_str(&cache)?;
        return Ok(GetAuthorsResponse {
            authors: cached_authors,
        });
    }

    let all_authors: Vec<Model_Author> =
        sqlx::query_as::<_, Model_Author>("SELECT * FROM author LIMIT $1 OFFSET $2")
            .bind(pages)
            .bind(pages_size)
            .fetch_all(pool)
            .await?;

    let proto_authors: Vec<ProtoAuthor> = all_authors
    .into_iter()
    .map(|a| ProtoAuthor {
        author_id: a.author_id,
        name: a.name,
    })
    .collect();

    let serialized = serde_json::to_string(&proto_authors)?;
    let _: () = redis.set_ex(authors_cache.clone(), serialized, cache_ttl_seconds).await?;

    Ok(GetAuthorsResponse {
        authors: proto_authors
    })
}

pub async fn register_author(
    pool: &PgPool,
    params: ProtoAuthor,
) -> Result<AddAuthorResponse, sqlx::Error> {
    let new_author = sqlx::query_as::<_, Model_Author>(
        "INSERT INTO author (author_id, name)
        VALUES ($1, $2)
        RETURNING *",
    )
    .bind(params.author_id)
    .bind(&params.name)
    .fetch_one(pool)
    .await?;

    Ok(AddAuthorResponse {
        author: Some(ProtoAuthor {
            author_id: new_author.author_id,
            name: new_author.name,
        }),
    })
}

pub async fn edit_author(
    pool: &PgPool,
    author_id: i64,
    params: UpdateAuthorRequest,
) -> Result<UpdateAuthorResponse, sqlx::Error> {
    let mut set_clauses = vec![];
    let mut i = 1;

    macro_rules! add_clause {
        ($field:expr, $name:expr, $clauses:expr, $i:expr) => {
            if $field.is_some() {
                $clauses.push(format!("{} = ${}", $name, $i));
                $i += 1;
            }
        };
    }

    add_clause!(params.name, "name", set_clauses, i);

    if set_clauses.is_empty() {
        return Err(sqlx::Error::Protocol("No fields to update".to_string()));
    }

    let query = format!(
        "UPDATE author SET {} WHERE author_id = ${} RETURNING *",
        set_clauses.join(", "),
        i
    );

    let mut q = sqlx::query_as::<_, Model_Author>(&query);

    if let Some(name) = &params.name {
        q = q.bind(name);
    }

    let edited_author = q.bind(author_id).fetch_all(pool).await?;

    Ok(UpdateAuthorResponse {
        authors: edited_author
            .into_iter()
            .map(|a| ProtoAuthor {
                author_id: a.author_id,
                name: a.name,
            })
            .collect(),
    })
}

pub async fn delete_author(
    pool: &PgPool,
    params: DeleteAuthorRequest,
) -> Result<AuthorDeleteResponse, sqlx::Error> {
    let author_id = params
        .author_id
        .parse::<i64>()
        .map_err(|_| sqlx::Error::Protocol("Invalid author_id".to_string()))?;

    let deleted_author =
        sqlx::query_as::<_, Model_Author>("DELETE FROM author WHERE author_id = $1 RETURNING *")
            .bind(author_id)
            .fetch_optional(pool)
            .await?;

    match deleted_author {
        Some(_) => Ok(AuthorDeleteResponse {
            sucessful: true,
            message: "Author deleted!".to_string(),
        }),
        None => Ok(AuthorDeleteResponse {
            sucessful: false,
            message: format!("Author with author_id {} not found", author_id),
        }),
    }
}

pub async fn get_author(
    pool: &PgPool,
    params: GetAuthorRequest,
) -> Result<GetAuthorResponse, sqlx::Error> {
    let author_id = params
        .author_id
        .parse::<i64>()
        .map_err(|_| sqlx::Error::Protocol("Invalid author_id".to_string()))?;

    let get_author = sqlx::query_as::<_, Model_Author>("SELECT * FROM author WHERE author_id = $1")
        .bind(author_id)
        .fetch_optional(pool)
        .await?;

    match get_author {
        Some(author) => Ok(GetAuthorResponse {
            author: Some(ProtoAuthor {
                author_id: author.author_id,
                name: author.name,
            }),
        }),
        None => Err(sqlx::Error::Protocol("No author found".to_string())),
    }
}

pub async fn get_authors_by_name(
    pool: &PgPool,
    params: &GetAuthorsByNameRequest,
) -> Result<GetAuthorsResponse, sqlx::Error> {
    let name_pattern = format!("%{}%", params.name.to_lowercase());

    let all_authors: Vec<Model_Author> =
        sqlx::query_as::<_, Model_Author>("SELECT * FROM author WHERE LOWER(name) LIKE $1")
            .bind(&name_pattern)
            .fetch_all(pool)
            .await?;

    Ok(GetAuthorsResponse {
        authors: all_authors
            .into_iter()
            .map(|a| ProtoAuthor {
                author_id: a.author_id,
                name: a.name,
            })
            .collect(),
    })
}

fn normalize_pagination(page_number: Option<i64>, page_size: Option<i64>) -> (i64, i64) {
    let page_number = page_number.unwrap_or(DEFAULT_PAGE_NUMBER).max(1);

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).max(1);
    let offset = (page_number - 1) * page_size;

    (page_size, offset)
}
