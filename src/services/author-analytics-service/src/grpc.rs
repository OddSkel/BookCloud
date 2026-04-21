pub mod contracts {
    pub mod common {
        tonic::include_proto!("gateway.common");
    }
    pub mod author_analytics {
        tonic::include_proto!("gateway.authoranalytics");
    }
}