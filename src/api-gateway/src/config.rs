use std::env;

pub struct AppConfig {
    pub service_name: String,
    pub host: String,
    pub api_port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            service_name: env::var("SERVICE_NAME").unwrap_or_else(|_| "api-gateway".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            api_port: read_port("API_PORT", 8080),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.api_port)
    }
}

fn read_port(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default)
}
