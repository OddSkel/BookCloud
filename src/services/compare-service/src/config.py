import os
from dataclasses import dataclass


@dataclass(frozen=True)
class AppConfig:
    service_name: str
    host: str
    grpc_port: int
    book_catalog_grpc_url: str
    author_catalog_grpc_url: str
    rating_catalog_grpc_url: str

    @classmethod
    def from_env(cls) -> "AppConfig":
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
        )

    @property
    def bind_address(self) -> str:
        return f"{self.host}:{self.grpc_port}"
