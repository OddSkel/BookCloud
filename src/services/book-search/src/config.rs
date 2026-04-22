use std::{
    env,
    net::{AddrParseError, SocketAddr},
};

pub struct AppConfig {
    pub service_name: String,
    pub host: String,
    pub grpc_port: u16,
    pub book_catalog_grpc_url: String,
    pub search_page_size: i32,
    pub search_max_pages: i32,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            service_name: "book-search".to_string(),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            grpc_port: read_port("GRPC_PORT", 50054),
            book_catalog_grpc_url: env::var("BOOK_CATALOG_GRPC_URL")
                .unwrap_or_else(|_| "http://book-catalog:50051".to_string()),
            search_page_size: read_i32("BOOK_SEARCH_PAGE_SIZE", 1000),
            search_max_pages: read_i32("BOOK_SEARCH_MAX_PAGES", 100),
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

fn read_i32(key: &str, default: i32) -> i32 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(default)
}
