pub mod contracts {
    pub mod common {
        #![allow(dead_code)]
        tonic::include_proto!("gateway.common");
    }

    pub mod rating_catalog {
        tonic::include_proto!("gateway.ratingcatalog");
    }
}
