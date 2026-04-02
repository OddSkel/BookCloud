use std::env;

pub struct AppConfig {
    pub service_name: String,
    pub grpc_port:    u16,
    pub http_port:    u16,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "author-analytics-service".to_string()),
            grpc_port: env::var("GRPC_PORT")
                .unwrap_or_else(|_| "50056".to_string())
                .parse().expect("GRPC_PORT must be a number"),
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8086".to_string())
                .parse().expect("HTTP_PORT must be a number"),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
        }
    }

    pub fn grpc_address(&self) -> String { format!("0.0.0.0:{}", self.grpc_port) }
    pub fn http_address(&self) -> String { format!("0.0.0.0:{}", self.http_port) }
}