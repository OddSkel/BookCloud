use std::{
    env,
    net::{AddrParseError, SocketAddr},
};

pub struct AppConfig {
    pub service_name: String,
    pub host: String,
    pub grpc_port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            service_name: "book-catalog".to_string(),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            grpc_port: read_port("GRPC_PORT", 50051),
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
