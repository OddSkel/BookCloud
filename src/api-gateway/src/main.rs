use actix_web::{web, App, HttpServer, HttpResponse};
use actix_web::web::PathConfig;
use serde_json::json;
use config::AppConfig;
use grpc::GrpcRegistry;
use routes::init_routes;

mod utils;
mod config;
mod grpc;
mod handlers;
mod routes;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let config = AppConfig::from_env();
    let address = config.bind_address();
    let grpc_registry = web::Data::new(GrpcRegistry::from_env());

    println!("{} started on {}", config.service_name, address);

    HttpServer::new(move || {
        App::new()
            .app_data(grpc_registry.clone())
            .app_data(
                PathConfig::default().error_handler(|err, _req| {
                    actix_web::error::InternalError::from_response(
                        err,
                        HttpResponse::BadRequest().json(json!({
                            "error": "Invalid path parameter"
                        })),
                    )
                    .into()
                }),
            )
            .configure(init_routes)
    })
    .bind(address)?
    .run()
    .await
}
