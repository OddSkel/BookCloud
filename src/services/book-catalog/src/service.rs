use tonic::{Request, Response, Status};

use crate::grpc::contracts::{
    book_catalog::{
        book_catalog_grpc_server::BookCatalogGrpc,
        AddBookRequest, AddBookResponse,
        DeleteBookRequest, DeleteBookResponse,
        GetBookRequest, GetBookResponse,
        GetBooksRequest, GetBooksResponse,
        UpdateBookRequest, UpdateBookResponse,
    },
    common::{HealthCheckRequest, HealthCheckResponse},
};

use crate::domain::book_service::BookService;

pub struct BookCatalogService {
    service_name: String,
    book_service: BookService,
    pool: PgPool,
}

impl BookCatalogService {
    pub fn new(service_name: String, pool: PgPool) -> Self {
        Self {
            service_name,
            book_service: BookService::new(),
            pool,
        }
    }
}

#[tonic::async_trait]
impl BookCatalogGrpc for BookCatalogService {

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            service: self.service_name.clone(),
            status: "ok".to_string(),
        }))
    }

    async fn get_books(
        &self,
        _request: Request<GetBooksRequest>,
    ) -> Result<Response<GetBooksResponse>, Status> {

        let books = self.book_service
            .get_books()
            .await
            .map_err(Status::internal)?;

        Ok(Response::new(GetBooksResponse {
            books: books.into_iter().map(|b| b.into()).collect(),
        }))
    }

    async fn get_book(
        &self,
        request: Request<GetBookRequest>,
    ) -> Result<Response<GetBookResponse>, Status> {

        let req = request.into_inner();

        let book = self.book_service
            .get_book(req.id)
            .await
            .map_err(Status::not_found)?;

        Ok(Response::new(GetBookResponse {
            book: Some(book.into()),
        }))
    }

    async fn add_book(
        &self,
        request: Request<AddBookRequest>,
    ) -> Result<Response<AddBookResponse>, Status> {

        let req = request.into_inner();

        let book = self.book_service
            .add_book(
                req.name,
                req.author,
                req.genre,
                req.year_published,
                req.isbn,
                if req.summary.is_empty() { None } else { Some(req.summary) },
            )
            .await
            .map_err(Status::invalid_argument)?;

        Ok(Response::new(AddBookResponse {
            book: Some(book.into()),
        }))
    }

    async fn update_book(
        &self,
        request: Request<UpdateBookRequest>,
    ) -> Result<Response<UpdateBookResponse>, Status> {

        let req = request.into_inner();

        let book = self.book_service
            .update_book(
                req.id,
                req.name,
                req.author,
                req.genre,
                req.year_published,
                req.isbn,
                if req.summary.is_empty() { None } else { Some(req.summary) },
            )
            .await
            .map_err(Status::internal)?;

        Ok(Response::new(UpdateBookResponse {
            book: Some(book.into()),
        }))
    }

    async fn delete_book(
        &self,
        request: Request<DeleteBookRequest>,
    ) -> Result<Response<DeleteBookResponse>, Status> {

        let req = request.into_inner();

        self.book_service
            .delete_book(req.id)
            .await
            .map_err(Status::internal)?;

        Ok(Response::new(DeleteBookResponse {
            success: true,
        }))
    }
}