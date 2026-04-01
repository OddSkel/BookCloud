pub mod contracts {
    pub mod common {
        tonic::include_proto!("common");
    }

    pub mod author_catalog {
        tonic::include_proto!("author_catalog");
    }
}
