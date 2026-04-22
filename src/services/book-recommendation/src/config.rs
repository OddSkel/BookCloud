use std::{
    env,
    net::{AddrParseError, SocketAddr},
};

pub struct AppConfig {
    pub service_name: String,
    pub host: String,
    pub grpc_port: u16,
    pub db_url: String,
    pub genre_analysis_grpc_url: String,
    pub book_catalog_grpc_url: String,
    pub rating_catalog_grpc_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            service_name: "book-recommendation".to_string(),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            grpc_port: read_port("GRPC_PORT", 50056),
            db_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            genre_analysis_grpc_url: env::var("GENRE_ANALYSIS_GRPC_URL")
                .unwrap_or_else(|_| "http://genre-analysis-service:50055".to_string()),
            book_catalog_grpc_url: env::var("BOOK_CATALOG_GRPC_URL")
                .unwrap_or_else(|_| "http://book-catalog:50051".to_string()),
            rating_catalog_grpc_url: env::var("RATING_CATALOG_GRPC_URL")
                .unwrap_or_else(|_| "http://rating-catalog:50053".to_string()),
        }
    }

    pub fn grpc_address(&self) -> Result<SocketAddr, AddrParseError> {
        format!("{}:{}", self.host, self.grpc_port).parse()
    }
}

fn read_port(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default)
}

