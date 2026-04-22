pub mod contracts {
    pub mod common {
        tonic::include_proto!("gateway.common");
    }

    pub mod book_recommendation {
        #![allow(dead_code)]
        tonic::include_proto!("gateway.book_recommendation");
    }

    pub mod book_catalog {
        #![allow(dead_code)]
        tonic::include_proto!("gateway.bookcatalog");
    }

    pub mod genre_service {
        #![allow(dead_code)]
        tonic::include_proto!("gateway.genreservice");
    }

    pub mod rating_catalog {
        #![allow(dead_code)]
        tonic::include_proto!("gateway.ratingcatalog");
    }
}
