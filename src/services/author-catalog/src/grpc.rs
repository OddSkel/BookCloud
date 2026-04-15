pub mod contracts {
    pub mod common {
        tonic::include_proto!("gateway.common");
    }

    pub mod author_catalog {
        tonic::include_proto!("gateway.authorcatalog");
    }
}
