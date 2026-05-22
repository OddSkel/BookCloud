# Unit Tests

They are done inside `service.rs`

Here's a breakdown of all 18 tests, grouped by what they cover:

## `rank_authors` (4 tests)
- **`test_rank_authors_by_avg_rating_orders_correctly`** — Bob (4.8 avg) ranks above Alice (3.75 avg)
- **`test_rank_authors_by_total_ratings_orders_correctly`** — Bob (200 ratings) ranks above Alice (150)
- **`test_rank_authors_empty_input_returns_empty_list`** — no authors in → empty list out, no crash
- **`test_rank_authors_single_author_returns_one_entry`** — single author returns correctly

## `authors_consistency` (4 tests)
- **`test_consistency_std_dev_computed_correctly`** — Alice with ratings [4.5, 3.0] → std_dev=0.75, avg=3.75
- **`test_consistency_single_book_author_has_zero_std_dev`** — one book → std_dev=0.0, consistency_score=1.0
- **`test_consistency_rank_assigned_correctly`** — Bob (std=0, most consistent) gets rank=1, Alice gets rank=2
- **`test_consistency_empty_input_returns_empty_list`** — no input → empty list, no crash

## `authors_growth` (5 tests)
- **`test_growth_excludes_single_book_authors`** — Bob (1 book) must be absent from results
- **`test_growth_score_is_latest_minus_first`** — Alice [2018→4.5, 2020→3.0] → growth_score=-1.5, first_rating=4.5, latest_rating=3.0
- **`test_growth_positive_score_when_improving`** — Carol [2018→3.0, 2022→5.0] → growth_score=+2.0
- **`test_growth_ranked_descending_by_growth_score`** — Carol (+2.0) ranks above Alice (-1.5)
- **`test_growth_empty_input_returns_empty_list`** — no input → empty list, no crash

## `author_performance` / utils (5 tests)
- **`test_pearson_correlation_insufficient_data_returns_zero`** — single data point → 0.0, no crash
- **`test_pearson_correlation_constant_values_returns_zero`** — all same values → 0.0, no division by zero
- **`test_pearson_correlation_perfect_positive`** — perfectly correlated series → ~1.0
- **`test_pearson_correlation_perfect_negative`** — inversely correlated series → ~-1.0
- **`test_interpret_correlation_covers_all_bands`** — verifies all 7 thresholds in `interpret_correlation()` return the correct label string