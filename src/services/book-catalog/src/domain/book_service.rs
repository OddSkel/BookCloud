use crate::models::book::Book;

pub struct BookService {
    books: Vec<Book>, // mock storage (temporary)
}

impl BookService {
    pub fn new() -> Self {
        Self {
            books: vec![],
        }
    }

    pub async fn get_books(&self) -> Result<Vec<Book>, String> {
        Ok(self.books.clone())
    }

    pub async fn get_book(&self, id: String) -> Result<Book, String> {
        self.books
            .iter()
            .find(|b| b.id == id)
            .cloned()
            .ok_or_else(|| "Book not found".to_string())
    }

    pub async fn add_book(
        &self,
        name: String,
        author: String,
        genre: String,
        year_published: i32,
        isbn: String,
        summary: Option<String>,
    ) -> Result<Book, String> {

        if name.is_empty() || author.is_empty() {
            return Err("Name and author are required".into());
        }

        let book = Book {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            author,
            genre,
            year_published,
            isbn,
            summary,
        };

        Ok(book)
    }

    pub async fn update_book(
        &self,
        id: String,
        name: String,
        author: String,
        genre: String,
        year_published: i32,
        isbn: String,
        summary: Option<String>,
    ) -> Result<Book, String> {

        let book = Book {
            id,
            name,
            author,
            genre,
            year_published,
            isbn,
            summary,
        };

        Ok(book)
    }

    pub async fn delete_book(&self, _id: String) -> Result<(), String> {
        Ok(())
    }
}