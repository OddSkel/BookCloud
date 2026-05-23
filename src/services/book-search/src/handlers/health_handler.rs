use crate::grpc::contracts::common::{HealthCheckRequest, HealthCheckResponse};

pub async fn health_check(service_name: HealthCheckRequest) -> HealthCheckResponse {
    HealthCheckResponse {
        service: service_name.service,
        status: "OK".to_string(),
    }
}
