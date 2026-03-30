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
                gender: a.gender,
                year_born: a.year_born,
                books_published: a.books_published,
                year_death: a.year_death,
            }
        ).collect(),
    })
}

pub async fn register_author(pool: &PgPool, params: ProtoAuthor) -> Result<AuthorResponse, sqlx:: Error> {
    let new_author = sqlx::query_as::<_, Model_Author>(
        "INSERT INTO authors (name gender year_born books_published year_death)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *"
    )
    .bind(&params.name)
    .bind(&params.gender)
    .bind(&params.year_born)
    .bind(&params.books_published)
    .bind(&params.year_death)
    .fetch_one(pool)
    .await?;

    Ok(
        AuthorResponse
        { author: Some(
            ProtoAuthor{
                id: new_author.id,
                name: new_author.name,
                gender: new_author.gender,
                year_born: new_author.year_born,
                books_published: new_author.books_published,
                year_death: new_author.year_death,
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
    add_clause!(params.gender, "gender", set_clauses, i);
    add_clause!(params.year_born, "year_born", set_clauses, i);
    add_clause!(params.books_published, "books_published", set_clauses, i);
    add_clause!(params.year_death, "year_death", set_clauses, i);

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
    if let Some(gender) = &params.gender { q = q.bind(gender); }
    if let Some(year_born) = params.year_born { q = q.bind(year_born); }
    if let Some(books_published) = params.books_published { q = q.bind(books_published); }
    if let Some(year_death) = params.year_death { q = q.bind(year_death); }

    let edited_author = q.bind(id).fetch_one(pool).await?;

    Ok(
        AuthorResponse
        { author: Some(
            ProtoAuthor{
                id: edited_author.id,
                name: edited_author.name,
                gender: edited_author.gender,
                year_born: edited_author.year_born,
                books_published: edited_author.books_published,
                year_death: edited_author.year_death,
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
                gender: author.gender,
                year_born: author.year_born,
                books_published: author.books_published,
                year_death: author.year_death,
            }),
        }),
        None => Err(sqlx::Error::Protocol("No author found".to_string())),
    }
}