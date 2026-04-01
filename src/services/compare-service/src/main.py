import asyncio
import logging

import grpc
from dotenv import load_dotenv

from src.config import AppConfig
from src.grpc import add_compare_service_to_server
from src.service import CompareService


async def serve() -> None:
    load_dotenv()
    logging.basicConfig(level=logging.INFO)

    config = AppConfig.from_env()
    server = grpc.aio.server()
    add_compare_service_to_server(CompareService(config), server)
    server.add_insecure_port(config.bind_address)

    logging.info("%s gRPC started on %s", config.service_name, config.bind_address)

    await server.start()
    await server.wait_for_termination()


if __name__ == "__main__":
    asyncio.run(serve())
