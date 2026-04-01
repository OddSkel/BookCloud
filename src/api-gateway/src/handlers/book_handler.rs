use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;

use crate::grpc::{BookAddPayload, GrpcRegistry};

#[derive(Deserialize)]
pub struct UpdateBookQuery {
    #[serde(rename = "Editor")]
    pub editor: Option<String>,
    #[serde(rename = "Year edited")]
    pub year_edited: Option<i32>,
}

pub async fn get_books(registry: web::Data<GrpcRegistry>) -> impl Responder {
    match registry.get_books(None, None).await {
        Ok(books) => HttpResponse::Ok().json(books),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

pub async fn get_book(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.get_book(&path.into_inner()).await {
        Ok(book) => HttpResponse::Ok().json(book),
        Err(e) => HttpResponse::NotFound().json(json!({ "error": e })),
    }
}

pub async fn add_book(
    registry: web::Data<GrpcRegistry>,
    body: web::Json<BookAddPayload>,
) -> impl Responder {
    match registry.add_book(body.into_inner()).await {
        Ok(book) => HttpResponse::Created().json(book),
        Err(e) => HttpResponse::UnprocessableEntity().json(json!({ "error": e })),
    }
}

pub async fn update_book(
    registry: web::Data<GrpcRegistry>,
    req: HttpRequest,
    body: web::Json<BookAddPayload>,
) -> impl Responder {
    let isbn = req
        .headers()
        .get("ISBN")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    match registry.update_book(isbn, body.into_inner()).await {
        Ok(books) => HttpResponse::Ok().json(books),
        Err(e) => HttpResponse::UnprocessableEntity().json(json!({ "error": e })),
    }
}

pub async fn delete_book(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.delete_book(&path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::NotFound().json(json!({ "error": e })),
    }
}
