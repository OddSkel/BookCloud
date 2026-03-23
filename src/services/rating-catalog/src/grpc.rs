pub mod contracts {
    pub mod common {
        tonic::include_proto!("gateway.common");
    }

    pub mod rating_catalog {
        tonic::include_proto!("gateway.ratingcatalog");
    }
}
