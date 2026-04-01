pub mod contracts {
    pub mod common {
        tonic::include_proto!("common");
    }

    pub mod book_recommendation {
        #![allow(dead_code)]
        tonic::include_proto!("book_recommendation");
    }
}
