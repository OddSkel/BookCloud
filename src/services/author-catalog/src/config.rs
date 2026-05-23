use std::env;


pub struct AppConfig {
    pub service_name: String,
    pub host: String,
    pub grpc_port: u16,
    pub redis_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let grpc_port = read_port("GRPC_PORT", 50052);
        let redis_url =  std::env::var("REDIS_URL").expect("REDIS_URL must be set");

        println!("DEBUG: HOST={}, GRPC_PORT={}", host, grpc_port);

        Self {
            service_name: "author-catalog".to_string(),
            host,
            grpc_port,
            redis_url,
        }
    }

    pub fn grpc_address_string(&self) -> String {
        format!("{}:{}", self.host, self.grpc_port)
    }
}

fn read_port(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default)
}
