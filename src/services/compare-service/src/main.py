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
    compare_service = CompareService(config)

    add_compare_service_to_server(compare_service, server)
    server.add_insecure_port(config.bind_address)

    try:
        await server.start()
        logging.info("%s gRPC started on %s", config.service_name, config.bind_address)

        await server.wait_for_termination()
    finally:
        await compare_service.close()


if __name__ == "__main__":
    asyncio.run(serve())