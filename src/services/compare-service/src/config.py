import os
from dataclasses import dataclass


def _env_bool(name: str, default: bool = False) -> bool:
    value = os.getenv(name)
    if value is None:
        return default
    return value.strip().lower() in ("true", "1", "yes", "y", "on")


@dataclass(frozen=True)
class AppConfig:
    service_name: str
    host: str
    grpc_port: int

    book_catalog_grpc_url: str
    author_catalog_grpc_url: str
    rating_catalog_grpc_url: str
    genre_analysis_grpc_url: str

    book_page_size: int
    rating_page_size: int
    genre_page_size: int
    parallel_requests: int

    # Local in-process cache for base datasets.
    cache_ttl_seconds: int

    # Redis response cache for final calculated JSON responses.
    redis_url: str | None
    response_cache_enabled: bool
    response_cache_ttl_seconds: int

    @classmethod
    def from_env(cls) -> "AppConfig":
        redis_url = os.getenv("REDIS_URL")

        return cls(
            service_name=os.getenv("SERVICE_NAME", "compare-service"),
            host=os.getenv("HOST", "0.0.0.0"),
            grpc_port=int(os.getenv("GRPC_PORT", "50054")),

            book_catalog_grpc_url=os.getenv(
                "BOOK_CATALOG_GRPC_URL",
                "http://book-catalog:50051",
            ),
            author_catalog_grpc_url=os.getenv(
                "AUTHOR_CATALOG_GRPC_URL",
                "http://author-catalog:50052",
            ),
            rating_catalog_grpc_url=os.getenv(
                "RATING_CATALOG_GRPC_URL",
                "http://rating-catalog:50053",
            ),
            genre_analysis_grpc_url=os.getenv(
                "GENRE_ANALYSIS_GRPC_URL",
                "http://genre-analysis-service:50055",
            ),

            book_page_size=int(os.getenv("BOOK_PAGE_SIZE", "2000")),
            rating_page_size=int(os.getenv("RATING_PAGE_SIZE", "2000")),
            genre_page_size=int(os.getenv("GENRE_PAGE_SIZE", "4000")),
            parallel_requests=int(os.getenv("PARALLEL_REQUESTS", "4")),

            cache_ttl_seconds=int(os.getenv("CACHE_TTL_SECONDS", "86400")),

            redis_url=redis_url if redis_url else None,
            response_cache_enabled=_env_bool("RESPONSE_CACHE_ENABLED", True),
            response_cache_ttl_seconds=int(
                os.getenv("RESPONSE_CACHE_TTL_SECONDS", "900")
            ),

        )

    @property
    def bind_address(self) -> str:
        return f"{self.host}:{self.grpc_port}"