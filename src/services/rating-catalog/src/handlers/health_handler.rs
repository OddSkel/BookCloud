
pub fn health(service_name: &str) -> (String, String) {
    (service_name.to_string(), "ok".to_string())
}