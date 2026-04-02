import logging

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
from src.grpc import (
    compare_filters_from_proto,
    compare_service_pb2,
    compare_service_pb2_grpc,
)


LOGGER = logging.getLogger(__name__)


class CompareService(compare_service_pb2_grpc.CompareServiceGrpcServicer):
    def __init__(self, config) -> None:
        self.config = config
        self.catalog_cache = CatalogDataCache(config)

    async def close(self) -> None:
        await self.catalog_cache.close()

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

    async def _handle_json_rpc(self, context, request, handler, operation_name: str):
        filters = request.filters if request.HasField("filters") else None
        response = None

        try:
            response = await handler(
                self.catalog_cache,
                compare_filters_from_proto(filters),
                self.config,
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

        return compare_service_pb2.JsonPayloadResponse(json_payload=response.to_json())
