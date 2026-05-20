use std::{
    env,
    net::{AddrParseError, SocketAddr},
};

pub struct AppConfig {
    pub service_name: String,
    pub host: String,
    pub grpc_port: u16,
    pub redis_url: String,
    pub cache_ttl_seconds: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let cache_ttl_seconds = env::var("CACHE_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(604800);

        Self {
            service_name: "rating-catalog".to_string(),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            grpc_port: read_port("GRPC_PORT", 50053),
            redis_url: std::env::var("REDIS_URL").expect("REDIS_URL must be set"),
            cache_ttl_seconds,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_port_returns_default_when_unset() {
        let port = read_port("NONEXISTENT_VAR_12345", 8080);
        assert_eq!(port, 8080);
    }

    #[test]
    fn read_port_returns_default_on_invalid_value() {
        unsafe { std::env::set_var("TEST_PORT_INVALID", "not_a_number") };
        let port = read_port("TEST_PORT_INVALID", 3000);
        assert_eq!(port, 3000);
        unsafe { std::env::remove_var("TEST_PORT_INVALID") };
    }

    #[test]
    fn read_port_parses_valid_value() {
        unsafe { std::env::set_var("TEST_PORT_VALID", "5432") };
        let port = read_port("TEST_PORT_VALID", 8080);
        assert_eq!(port, 5432);
        unsafe { std::env::remove_var("TEST_PORT_VALID") };
    }

    #[test]
    fn grpc_address_formats_correctly() {
        let config = AppConfig {
            service_name: "test".into(),
            host: "127.0.0.1".into(),
            grpc_port: 50053,
            redis_url: "redis://localhost:6379".into(),
            cache_ttl_seconds: 3600,
        };
        let addr = config.grpc_address().unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:50053");
    }

    #[test]
    fn grpc_address_fails_on_invalid_host() {
        let config = AppConfig {
            service_name: "test".into(),
            host: String::new(),
            grpc_port: 50053,
            redis_url: "redis://localhost:6379".into(),
            cache_ttl_seconds: 3600,
        };
        assert!(config.grpc_address().is_err());
    }
}
