use sqlx::{PgPool};
use crate::models::author::Author as Model_Author;
use crate::grpc::contracts::author_catalog::{
    GetAuthorsRequest,
    GetAuthorsResponse,
    AddAuthorResponse,
    UpdateAuthorRequest,
    UpdateAuthorResponse,
    DeleteAuthorRequest,
    AuthorDeleteResponse,
    GetAuthorRequest,
    GetAuthorResponse,
    Author as ProtoAuthor
};

const DEFAULT_PAGE_SIZE: i64 = 10;
const DEFAULT_PAGE_NUMBER: i64 = 1;

pub async fn get_authors(pool: &PgPool, params: &GetAuthorsRequest) -> Result<GetAuthorsResponse, sqlx:: Error> {

    let (pages, pages_size) = normalize_pagination(params.page, params.page_size);

    let all_authors: Vec<Model_Author> = sqlx::query_as::<_, Model_Author>(
        "SELECT * FROM authors LIMIT $1 OFFSET $2"
    )
    .bind(pages)
    .bind(pages_size)
    .fetch_all(pool)
    .await?;

    Ok(GetAuthorsResponse {
        authors: all_authors.into_iter().map(
            |a| ProtoAuthor {
                id: a.id,
                name: a.name,
            }
        ).collect(),
    })
}

pub async fn register_author(pool: &PgPool, params: ProtoAuthor) -> Result<AddAuthorResponse, sqlx:: Error> {
    let new_author = sqlx::query_as::<_, Model_Author>(
        "INSERT INTO authors (name)
        VALUES ($1)
        RETURNING *"
    )
    .bind(&params.name)
    .fetch_one(pool)
    .await?;

    Ok(
        AddAuthorResponse
        { author: Some(
            ProtoAuthor{
                id: new_author.id,
                name: new_author.name,
            })
        })
}

pub async fn edit_author(pool: &PgPool, id: i32, params: UpdateAuthorRequest) -> Result<UpdateAuthorResponse, sqlx:: Error> {
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
        "UPDATE authors SET {} WHERE id = ${} RETURNING *",
        set_clauses.join(", "),
        i
    );

    let mut q = sqlx::query_as::<_, Model_Author>(&query);

    if let Some(name) = &params.name { q = q.bind(name); }

    let edited_author = q.bind(id).fetch_all(pool).await?;

    Ok(UpdateAuthorResponse{
        authors: edited_author.into_iter().map(
        |a| ProtoAuthor {
            id: a.id,
            name: a.name,
        }
    ).collect(),
    })
}

pub async fn delete_author(pool: &PgPool, params: DeleteAuthorRequest) -> Result<AuthorDeleteResponse, sqlx:: Error> {
    let id = params.author_id.parse::<i32>().map_err(|_| sqlx::Error::Protocol("Invalid author_id".to_string()))?;
    
    let deleted_author = sqlx::query_as::<_, Model_Author>(
        "DELETE FROM authors WHERE id = $1"
    ).bind(id)
    .fetch_optional(pool)
    .await?;


    match deleted_author {
        Some(_) => Ok(AuthorDeleteResponse {
            sucessful: true,
            message: "Author deleted!".to_string(),
        }),
        None => Ok(AuthorDeleteResponse {
            sucessful: false,
            message: format!("Author with id {} not found", id),
        }),
    }
}

pub async fn get_author(pool: &PgPool, params: GetAuthorRequest) -> Result<GetAuthorResponse, sqlx:: Error> {
    let id = params.author_id.parse::<i32>().map_err(|_| sqlx::Error::Protocol("Invalid author_id".to_string()))?;
    
    let get_author = sqlx::query_as::<_, Model_Author>(
        "SELECT * FROM authors WHERE id = $1"
    ).bind(id)
    .fetch_optional(pool)
    .await?;

    match get_author {
        Some(author) => Ok(GetAuthorResponse {
            author: Some(ProtoAuthor{
                id: author.id,
                name: author.name,
            }),
        }),
        None => Err(sqlx::Error::Protocol("No author found".to_string())),
    }
}

fn normalize_pagination(page_number: Option<i64>, page_size: Option<i64>) -> (i64, i64) {
    let page_number = page_number.unwrap_or(DEFAULT_PAGE_NUMBER).max(1);

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).max(1);
    let offset = (page_number - 1) * page_size;

    (page_size, offset)
}