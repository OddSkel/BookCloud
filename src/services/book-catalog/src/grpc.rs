pub mod contracts {
    pub mod common {
        tonic::include_proto!("gateway.common");
    }

    pub mod book_catalog {
        tonic::include_proto!("gateway.bookcatalog");
    }
}
