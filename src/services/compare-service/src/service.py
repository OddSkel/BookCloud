import logging
import time
from typing import Awaitable, Callable

import grpc

from src.catalog_cache import CatalogDataCache
from src.handlers.popularity_handler import (
    UnsupportedFilterError,
    get_correlation,
    get_eras,
    get_hidden_gems,
    get_popular_low_rated,
    get_publishing_growth,
)
from src.models.popularity import PopularityFilters
from src.grpc import (
    compare_filters_from_proto,
    compare_service_pb2,
    compare_service_pb2_grpc,
)
from src.response_cache import ResponseCache


LOGGER = logging.getLogger(__name__)


class CompareService(compare_service_pb2_grpc.CompareServiceGrpcServicer):
    def __init__(self, config) -> None:
        self.config = config
        self.catalog_cache = CatalogDataCache(config)
        self.response_cache = ResponseCache(
            redis_url=config.redis_url,
            ttl_seconds=config.response_cache_ttl_seconds,
            enabled=config.response_cache_enabled,
        )

    async def close(self) -> None:
        await self.catalog_cache.close()
        await self.response_cache.close()

    async def GetPopularLowRated(self, request, context):
        return await self._handle_json_rpc(
            context,
            request,
            get_popular_low_rated,
            "popular-low-rated",
        )

    async def GetHiddenGems(self, request, context):
        return await self._handle_json_rpc(
            context,
            request,
            get_hidden_gems,
            "hidden-gems",
        )

    async def GetCorrelation(self, request, context):
        return await self._handle_json_rpc(
            context,
            request,
            get_correlation,
            "correlation",
        )

    async def GetPublishingGrowth(self, request, context):
        return await self._handle_json_rpc(
            context,
            request,
            get_publishing_growth,
            "publishing-growth",
        )

    async def GetEras(self, request, context):
        return await self._handle_json_rpc(
            context,
            request,
            get_eras,
            "eras",
        )

    async def _cached_json_response(
        self,
        operation: str,
        filters: PopularityFilters,
        compute: Callable[[], Awaitable],
    ) -> str:
        key = self.response_cache.build_key(operation, filters)

        cached_json = await self.response_cache.get_json(key)
        if cached_json is not None:
            LOGGER.info("Redis response cache HIT operation=%s", operation)
            return cached_json

        LOGGER.info("Redis response cache MISS operation=%s", operation)

        start = time.monotonic()
        response = await compute()
        compute_elapsed = time.monotonic() - start
        LOGGER.info(
            "Computed %s in %.2fs",
            operation,
            compute_elapsed,
        )

        response_json = response.to_json()

        await self.response_cache.set_json(key, response_json)

        return response_json

    async def _handle_json_rpc(self, context, request, handler, operation_name: str):
        filters = request.filters if request.HasField("filters") else None
        proto_filters = compare_filters_from_proto(filters)

        try:
            response_json = await self._cached_json_response(
                operation_name,
                proto_filters,
                lambda: handler(
                    self.catalog_cache,
                    proto_filters,
                    self.config,
                ),
            )
        except UnsupportedFilterError as exc:
            await context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(exc))
            raise AssertionError("context.abort should terminate the RPC")
        except grpc.RpcError as exc:
            LOGGER.exception("upstream gRPC error while computing %s", operation_name)
            details = exc.details() or exc.code().name
            await context.abort(
                grpc.StatusCode.UNAVAILABLE,
                f"Failed to fetch upstream catalog data: {details}",
            )
            raise AssertionError("context.abort should terminate the RPC")
        except Exception as exc:
            LOGGER.exception("unexpected compare-service failure in %s", operation_name)
            await context.abort(grpc.StatusCode.INTERNAL, str(exc))
            raise AssertionError("context.abort should terminate the RPC")

        return compare_service_pb2.JsonPayloadResponse(json_payload=response_json)