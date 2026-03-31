use sqlx::{PgPool};
use crate::models::author::Author as Model_Author;
use crate::grpc::contracts::author_catalog::{AuthorsResponse, AuthorId, AuthorDeleteResponse, AuthorResponse, Author as ProtoAuthor, AuthorEditParameters};

pub async fn get_authors(pool: &PgPool) -> Result<AuthorsResponse, sqlx:: Error> {

    let all_authors: Vec<Model_Author> = sqlx::query_as::<_, Model_Author>(
        "SELECT * FROM authors"
    )
    .fetch_all(pool)
    .await?;

    Ok(AuthorsResponse {
        authors: all_authors.into_iter().map(
            |a| ProtoAuthor {
                id: a.id,
                name: a.name,
            }
        ).collect(),
    })
}

pub async fn register_author(pool: &PgPool, params: ProtoAuthor) -> Result<AuthorResponse, sqlx:: Error> {
    let new_author = sqlx::query_as::<_, Model_Author>(
        "INSERT INTO authors (name)
        VALUES ($1)
        RETURNING *"
    )
    .bind(&params.name)
    .fetch_one(pool)
    .await?;

    Ok(
        AuthorResponse
        { author: Some(
            ProtoAuthor{
                id: new_author.id,
                name: new_author.name,
            })
        })
}

pub async fn edit_author(pool: &PgPool, id: i32, params: AuthorEditParameters) -> Result<AuthorResponse, sqlx:: Error> {
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

    let edited_author = q.bind(id).fetch_one(pool).await?;

    Ok(
        AuthorResponse
        { author: Some(
            ProtoAuthor{
                id: edited_author.id,
                name: edited_author.name,
            })
        })
}

pub async fn delete_author(pool: &PgPool, params: AuthorId) -> Result<AuthorDeleteResponse, sqlx:: Error> {
    let deleted_author = sqlx::query_as::<_, Model_Author>(
        "DELETE FROM authors WHERE id = $1"
    ).bind(params.id)
    .fetch_optional(pool)
    .await?;


    match deleted_author {
        Some(_) => Ok(AuthorDeleteResponse {
            sucessful: true,
            message: "Author deleted!".to_string(),
        }),
        None => Ok(AuthorDeleteResponse {
            sucessful: false,
            message: format!("Author with id {} not found", params.id),
        }),
    }
}

pub async fn get_author(pool: &PgPool, params: AuthorId) -> Result<AuthorResponse, sqlx:: Error> {
    
    let get_author = sqlx::query_as::<_, Model_Author>(
        "SELECT * FROM authors WHERE id = $1 RETURNING *"
    ).bind(params.id)
    .fetch_optional(pool)
    .await?;

    match get_author {
        Some(author) => Ok(AuthorResponse {
            author: Some(ProtoAuthor{
                id: author.id,
                name: author.name,
            }),
        }),
        None => Err(sqlx::Error::Protocol("No author found".to_string())),
    }
}