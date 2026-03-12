# Phase 3 – Functional Requirements and Application Architecture
Group 2

## 1. Functional Requirements

### Service 1

#### FR1 – 

##### Description

##### Inputs

##### Processing

##### Outputs

---

#### FR2 – 

##### Description

##### Inputs

##### Processing

##### Outputs

## 2. Application Architecture

### 2.1 System Components

### 2.2 Component Responsibilities

#### Microservices

#### Database

--

## 3. Application Architecture Diagram

```mermaid
graph TB

    client[Client] --> gateway[API Gateway]

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
    gateway --> search_api
    gateway --> genre_api
    gateway --> compare_api
    gateway --> author_api

    %% Bases de dados locais
    genre_api --> genre_db
    book_api --> book_db
    author_catalog_api --> author_catalog_db
    rating_api --> rating_db

    %% Dependências entre serviços
    search_api --> book_api
    search_api --> author_catalog_api

    compare_api --> book_api
    compare_api --> author_catalog_api
    compare_api --> rating_api

    genre_api --> book_api
    genre_api --> rating_api

    author_api --> author_catalog_api
    author_api --> book_api
    author_api --> rating_api
```