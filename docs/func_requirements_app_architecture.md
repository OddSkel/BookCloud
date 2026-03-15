# Phase 3 – Functional Requirements and Application Architecture
Group 2

## 1. Functional Requirements

### Service 1 — Genre Intelligence & Market Trends

#### FR1 – Rank genres by rating or popularity

##### Description
The system shall allow users to retrieve a ranking of book genres based on either:
- average rating, or
- total number of ratings (popularity).

This requirement supports the identification of the most highly rated genres and the most popular genres in the dataset.

##### Inputs
- `rank_sort` (query parameter):
  - `rate_asc`
  - `rate_desc`
  - `pop_asc`
  - `pop_desc`

##### Processing
- The service receives the ranking request.
- It retrieves genre-related book and rating data.
- It aggregates the data by genre.
- It calculates:
  - average rating per genre, and/or
  - total number of ratings per genre.
- It sorts the results according to the requested ranking mode.

##### Outputs
- A JSON response containing:
  - genre name
  - rank
  - average rating
  - total ratings
  - total number of genres returned

**Related endpoint:** `GET /genres`

---

#### FR2 – Analyze genre growth or decline over time

##### Description
The system shall allow users to analyze how a specific genre evolves over time, identifying whether it is growing, stable or declining based on yearly aggregated data.

##### Inputs
- `genre_id` (path parameter)
- `year_from` (query parameter, optional)
- `year_to` (query parameter, optional)

##### Processing
- The service receives the selected genre and time interval.
- It retrieves all books associated with the genre within the given years.
- It aggregates the number of books and average yearly indicators for the selected genre.
- It produces a temporal view of genre evolution.

##### Outputs
- A JSON response containing:
  - genre identifier
  - genre name
  - selected time interval
  - yearly data points for genre evolution

**Related endpoint:** `GET /genres/{genre_id}/growth`

---

#### FR3 – Analyze genre popularity trends over time

##### Description
The system shall allow users to retrieve the popularity trend of a specific genre over time based on the number of ratings accumulated each year.

##### Inputs
- `genre_id` (path parameter)
- `year_from` (query parameter, optional)
- `year_to` (query parameter, optional)

##### Processing
- The service receives the genre and time range.
- It collects all books associated with that genre.
- It aggregates popularity indicators by year.
- It computes yearly popularity metrics such as total ratings.

##### Outputs
- A JSON response containing:
  - genre identifier
  - genre name
  - selected interval
  - yearly popularity data

**Related endpoint:** `GET /genres/{genre_id}/popularity`

---

### Service 2 — Smart Book Discovery & Recommendation

#### FR4 – Search books by title, author or summary keywords

##### Description
The system shall allow users to search books using different discovery criteria, including title, author name and keywords extracted from the book summary.

##### Inputs
- `Title` (query parameter, optional)
- `Author` (query parameter, optional)
- `Keywords` (query parameter, optional)

##### Processing
- The service receives one or more search parameters.
- It queries the book catalog and author catalog.
- It applies filtering based on title, author and summary keywords.
- It returns the matching books.

##### Outputs
- A JSON response containing the list of matching books and their metadata.

**Related endpoint:** `GET /searchBook`

---

#### FR5 – Recommend similar books

##### Description
The system shall allow users to obtain book recommendations based on:
- genre overlap,
- rating similarity,
- popularity profile.

##### Inputs
- `Genre` (query parameter, optional)
- `Rating` (query parameter, optional)
- `Popularity` (query parameter, optional)

##### Processing
- The service receives the recommendation criteria.
- It queries the catalog data and analytics data.
- It compares books using the selected similarity dimensions.
- It ranks the most relevant recommended books.

##### Outputs
- A JSON response containing a ranked list of recommended books.

**Related endpoint:** `GET /recommendation`

---

### Service 3 — Author Performance Analytics

#### FR6 – Rank authors by average rating or total number of ratings

##### Description
The system shall allow users to rank authors according to their average rating or their total number of ratings.

##### Inputs
- `Average Rating` (query parameter, optional)
- `Total number ratings` (query parameter, optional)

##### Processing
- The service receives the ranking request.
- It gathers author, book and rating data.
- It aggregates metrics per author.
- It sorts authors according to the chosen ranking criterion.

##### Outputs
- A JSON response containing ranked authors and their performance indicators.

**Related endpoint:** `GET /rank-author`

---

#### FR7 – Analyze author performance evolution over time

##### Description
The system shall allow users to evaluate the performance evolution of an author across time, combining quality and popularity indicators for the author’s books.

##### Inputs
- `author_name` (query parameter, optional)
- `author_id` (query parameter, optional)
- `pub_year_from` (query parameter, optional)
- `pub_year_to` (query parameter, optional)

##### Processing
- The service identifies the selected author.
- It retrieves the author’s books and related ratings.
- It computes time-based metrics such as quality score and popularity score.
- It builds a chronological evolution of the author’s performance.

##### Outputs
- A JSON response containing:
  - author identification
  - applied filters
  - statistical indicators
  - yearly or per-book evolution data

**Related endpoint:** `GET /author-performance-time`

---

#### FR8 – Identify consistent authors

##### Description
The system shall allow users to identify authors whose books show the most consistent rating patterns.

##### Inputs
- `author_name` (query parameter, optional)
- `author_id` (query parameter, optional)

##### Processing
- The service retrieves the ratings of books from the selected authors.
- It calculates consistency indicators, such as average rating and rating dispersion.
- It ranks authors according to their consistency score.

##### Outputs
- A JSON response containing authors ranked by consistency.

**Related endpoint:** `GET /authors-consistency`

---

#### FR9 – Identify authors with highest growth

##### Description
The system shall allow users to identify authors whose ratings improved the most over time.

##### Inputs
- `author_name` (query parameter, optional)
- `author_id` (query parameter, optional)

##### Processing
- The service retrieves rating history for the selected authors.
- It compares early and recent performance indicators.
- It computes a growth score.
- It ranks authors according to growth.

##### Outputs
- A JSON response containing authors ranked by growth score.

**Related endpoint:** `GET /authors-growth`

---

### Service 4 — Popularity vs Quality Analytics

#### FR10 – Identify very popular but low-rated books

##### Description
The system shall allow users to identify books that have very high popularity but relatively low rating, highlighting titles with a strong discrepancy between visibility and perceived quality.

##### Inputs
Possible filters include:
- `author_name`
- `author_id`
- `genre_name`
- `genre_id`
- `book_name`
- `book_isbn`
- `pub_year_from`
- `pub_year_to`
- `pub_year`
- `page`
- `page_size`
- `min_num_ratings`
- `max_num_ratings`
- `min_star_rating`
- `max_star_rating`

##### Processing
- The service filters the candidate books.
- It compares popularity indicators (`num_ratings`) with quality indicators (`star_rating`).
- It computes a discrepancy score.
- It ranks books with the highest discrepancy.

##### Outputs
- A paginated JSON response containing books classified as popular but low-rated.

**Related endpoint:** `GET /popular-low-rated`

---

#### FR11 – Identify hidden gems

##### Description
The system shall allow users to identify books with high ratings but relatively low popularity.

##### Inputs
Possible filters include:
- `author_name`
- `author_id`
- `genre_name`
- `genre_id`
- `book_name`
- `book_isbn`
- `pub_year_from`
- `pub_year_to`
- `pub_year`
- `page`
- `page_size`
- `min_num_ratings`
- `max_num_ratings`
- `min_star_rating`
- `max_star_rating`

##### Processing
- The service filters the candidate books.
- It compares popularity and rating indicators.
- It computes a discrepancy score.
- It ranks books with the smallest discrepancy in favor of quality.

##### Outputs
- A paginated JSON response containing hidden gem books.

**Related endpoint:** `GET /hidden-gems`

---

#### FR12 – Compute correlation between rating and popularity

##### Description
The system shall allow users to measure the statistical correlation between average star rating and number of ratings.

##### Inputs
- `method` (`pearson` or `spearman`)
- `genre_name` (optional)
- `genre_id` (optional)
- `pub_year_from` (optional)
- `pub_year_to` (optional)
- `pub_year` (optional)
- `min_num_ratings` (optional)

##### Processing
- The service filters the books according to the selected criteria.
- It extracts quality and popularity attributes.
- It applies the selected statistical correlation method.
- It interprets the resulting coefficient.

##### Outputs
- A JSON response containing:
  - correlation method
  - correlation coefficient
  - interpretation
  - sample size

**Related endpoint:** `GET /correlation`

---

#### FR13 – Analyze publishing growth over time

##### Description
The system shall allow users to study yearly publishing evolution based on number of books published, average rating and total ratings.

##### Inputs
- `genre_name` (optional)
- `genre_id` (optional)
- `pub_year_from` (optional)
- `pub_year_to` (optional)

##### Processing
- The service filters books by genre and publication interval.
- It groups results by publication year.
- It computes annual metrics such as number of books published and average ratings.

##### Outputs
- A JSON response containing yearly publishing growth metrics.

**Related endpoint:** `GET /publishing-growth`

---

#### FR14 – Compare classic and modern literature

##### Description
The system shall allow users to compare classic and modern books according to configurable publication year thresholds.

##### Inputs
- `genre_name` (optional)
- `genre_id` (optional)
- `classic_threshold` (optional)
- `modern_threshold` (optional)

##### Processing
- The service separates books into classic and modern eras.
- It aggregates metrics for each group.
- It compares number of books, average ratings and popularity indicators.

##### Outputs
- A JSON response containing comparative metrics for classic and modern literature.

**Related endpoint:** `GET /eras`

---

### Shared Catalog Functional Requirements

#### FR15 – Manage books

##### Description
The system shall support CRUD operations for books.

##### Inputs
- Book metadata such as:
  - name
  - ISBN
  - author
  - year published
  - editor
  - edition number
  - summary
  - genre
  - identifier

##### Processing
- Validate input data.
- Create, retrieve, update or delete book records.

##### Outputs
- JSON responses representing created, updated, retrieved or deleted book data.

**Related endpoints:**  
`GET /books`  
`POST /book`  
`GET /book/{book_id}`  
`PUT /book`  
`DELETE /book/{book_id}`

---

#### FR16 – Manage authors

##### Description
The system shall support CRUD operations for authors.

##### Inputs
- Author metadata such as:
  - name
  - gender
  - year born
  - year death
  - books published
  - identifier

##### Processing
- Validate input data.
- Create, retrieve, update or delete author records.

##### Outputs
- JSON responses representing created, updated, retrieved or deleted author data.

**Related endpoints:**  
`GET /authors`  
`POST /author`  
`GET /author/{author_id}`  
`PUT /author`  
`DELETE /author/{author_id}`

---

#### FR17 – Manage ratings

##### Description
The system shall support CRUD operations for ratings associated with books.

##### Inputs
- Rating metadata such as:
  - evaluation
  - critic/comment
  - identifier
  - associated book

##### Processing
- Validate input data.
- Create, retrieve, update or delete rating records.

##### Outputs
- JSON responses representing created, updated, retrieved or deleted rating data.

**Related endpoints:**  
`GET /ratings/{book_id}`  
`POST /rating/{book_id}`  
`GET /rating/{rating_id}`  
`PUT /rating/{book_id}`  
`DELETE /rating/{rating_id}`

---

## 2. Application Architecture

### 2.1 System Components

The application follows a microservices-based architecture organized into two main domains:

- **Catalog Services**
  - BookCatalog
  - AuthorCatalog
  - RatingCatalog

- **Analytics Services**
  - SearchService
  - GenreAnalysisService
  - CompareService
  - AuthorAnalyticsService

In addition, the system includes:
- **Client**
- **API Gateway**
- **Databases** associated with catalog services and genre data

### 2.2 Component Responsibilities

#### Microservices

**API Gateway**
- Acts as the single entry point for client requests.
- Routes incoming gRPC requests to the appropriate microservice.
- Simplifies access to the distributed system.

**BookCatalog**
- Manages book data.
- Supports CRUD operations for books.
- Provides book metadata to analytics services.

**AuthorCatalog**
- Manages author data.
- Supports CRUD operations for authors.
- Provides author information to analytics services.

**RatingCatalog**
- Manages rating data.
- Supports CRUD operations for ratings.
- Provides rating information to analytics services.

**SearchService**
- Implements book discovery and recommendation features.
- Uses data from BookCatalog and AuthorCatalog.
- Supports search by title, author and summary keywords.
- Generates recommendations based on genre, rating and popularity.

**GenreAnalysisService**
- Implements genre-related analytics.
- Computes genre rankings, growth and popularity trends.
- Uses book and rating data to generate aggregated genre metrics.

**CompareService**
- Implements popularity versus quality analytics.
- Identifies popular low-rated books and hidden gems.
- Computes correlation metrics and publishing growth.
- Supports classic versus modern comparisons.

**AuthorAnalyticsService**
- Implements author performance analytics.
- Ranks authors and evaluates consistency, growth and performance evolution.
- Uses author, book and rating information.

#### Database

The system uses separate databases for the core catalog services and genre-related persistence shown in the architecture diagram.

**Book Database**
- Stores book metadata ingested from the Goodreads dataset.

**Author Database**
- Stores author metadata and related author information.

**Rating Database**
- Stores ratings and review-related information associated with books.

**Genre Database**
- Stores genre entities and genre-related aggregated or managed data.

This separation improves modularity, service autonomy and maintainability.

--

## 3. Application Architecture Diagram

```mermaid
graph TB

    client[Client] --> |HTTP| gateway[API Gateway]

    subgraph analytics[Analytics Services]
        subgraph search_ms[SearchService]
            search_api[API]
        end

        subgraph genre_ms[GenreAnalysisService]
            genre_api[API]
            genre_db[(Database)]
        end

        subgraph compare_ms[CompareService]
            compare_api[API]
        end

        subgraph author_ms[AuthorAnalyticsService]
            author_api[API]
        end
    end

    subgraph catalogs[Catalog Services]
        subgraph book_ms[BookCatalog]
            book_api[API]
            book_db[(Database)]
        end

        subgraph author_catalog_ms[AuthorCatalog]
            author_catalog_api[API]
            author_catalog_db[(Database)]
        end

        subgraph rating_ms[RatingCatalog]
            rating_api[API]
            rating_db[(Database)]
        end
    end

    %% Gateway
    gateway -->|gRPC| search_api
    gateway -->|gRPC| genre_api
    gateway -->|gRPC| compare_api
    gateway -->|gRPC| author_api
    gateway -->|gRPC| book_api
    gateway -->|gRPC| author_catalog_api
    gateway -->|gRPC| rating_api

    %% Bases de dados locais
    genre_api --> genre_db
    book_api --> book_db
    author_catalog_api --> author_catalog_db
    rating_api --> rating_db

    %% Dependências entre serviços
    search_api -->|gRPC| book_api
    search_api -->|gRPC| author_catalog_api

    compare_api -->|gRPC| book_api
    compare_api -->|gRPC| author_catalog_api
    compare_api -->|gRPC| rating_api

    genre_api -->|gRPC| book_api
    genre_api -->|gRPC| rating_api

    author_api -->|gRPC| author_catalog_api
    author_api -->|gRPC| book_api
    author_api -->|gRPC| rating_api