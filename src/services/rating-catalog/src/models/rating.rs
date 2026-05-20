use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rating {
    pub book_isbn: i64,
    pub star_rating: f64,
    pub num_ratings: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_round_trips_through_json() {
        let rating = Rating {
            book_isbn: 42,
            star_rating: 4.5,
            num_ratings: 100,
        };
        let json = serde_json::to_string(&rating).unwrap();
        let deserialized: Rating = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.book_isbn, 42);
        assert_eq!(deserialized.star_rating, 4.5);
        assert_eq!(deserialized.num_ratings, 100);
    }

    #[test]
    fn rating_serializes_with_correct_field_names() {
        let rating = Rating {
            book_isbn: 1,
            star_rating: 3.8,
            num_ratings: 50,
        };
        let json = serde_json::to_string(&rating).unwrap();
        assert!(json.contains("book_isbn"));
        assert!(json.contains("star_rating"));
        assert!(json.contains("num_ratings"));
    }

    #[test]
    fn rating_handles_zero_values() {
        let rating = Rating {
            book_isbn: 0,
            star_rating: 0.0,
            num_ratings: 0,
        };
        let json = serde_json::to_string(&rating).unwrap();
        let deserialized: Rating = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.book_isbn, 0);
        assert_eq!(deserialized.star_rating, 0.0);
        assert_eq!(deserialized.num_ratings, 0);
    }

    #[test]
    fn rating_handles_large_values() {
        let rating = Rating {
            book_isbn: i64::MAX,
            star_rating: 5.0,
            num_ratings: i64::MAX,
        };
        let json = serde_json::to_string(&rating).unwrap();
        let deserialized: Rating = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.book_isbn, i64::MAX);
        assert_eq!(deserialized.star_rating, 5.0);
        assert_eq!(deserialized.num_ratings, i64::MAX);
    }
}
