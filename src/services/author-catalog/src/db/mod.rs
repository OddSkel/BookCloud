use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub async fn create_pool() -> Result<Pool<Postgres>, sqlx::Error> {
    let user = std::env::var("POSTGRES_USER").expect("POSTGRES_USER not defined.");
    let db = std::env::var("POSTGRES_DB").expect("POSTGRES_DB not defined.");
    let password = std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD not defined.");
    let host = std::env::var("POSTGRES_HOST").expect("POSTGRES_HOST not defined.");
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());

    let database_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        user, password, host, port, db
    );

    let pool: Pool<Postgres> = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Database connected successfully!");

    Ok(pool)
}
