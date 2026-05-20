import hashlib
import json
import logging
from dataclasses import asdict, is_dataclass
from typing import Any

import redis.asyncio as redis


LOGGER = logging.getLogger(__name__)


class ResponseCache:
    """
    Redis-backed cache for final JSON responses.

    This cache is intentionally best-effort:
    - if Redis is down, requests are still computed normally;
    - values stored in Redis are plain JSON strings;
    - cache keys are stable hashes of operation + filters.
    """

    def __init__(
        self,
        redis_url: str | None,
        ttl_seconds: int,
        enabled: bool = True,
    ) -> None:
        self._ttl_seconds = max(ttl_seconds, 0)
        self._enabled = enabled and bool(redis_url) and self._ttl_seconds > 0
        self._redis = (
            redis.from_url(redis_url, decode_responses=True)
            if self._enabled
            else None
        )

    @property
    def enabled(self) -> bool:
        return self._redis is not None

    async def close(self) -> None:
        if self._redis is not None:
            await self._redis.aclose()

    async def get_json(self, key: str) -> str | None:
        if self._redis is None:
            return None

        try:
            return await self._redis.get(key)
        except Exception:
            LOGGER.exception("Redis response cache get failed")
            return None

    async def set_json(self, key: str, value: str) -> None:
        if self._redis is None:
            return

        try:
            await self._redis.setex(key, self._ttl_seconds, value)
        except Exception:
            LOGGER.exception("Redis response cache set failed")

    @staticmethod
    def build_key(operation: str, filters: Any) -> str:
        payload = _stable_payload(filters)
        raw = json.dumps(payload, sort_keys=True, separators=(",", ":"))
        digest = hashlib.sha256(raw.encode("utf-8")).hexdigest()
        return f"compare:{operation}:{digest}"


def _stable_payload(value: Any) -> Any:
    if is_dataclass(value):
        return _stable_payload(asdict(value))

    if isinstance(value, dict):
        return {
            str(key): _stable_payload(item)
            for key, item in sorted(value.items(), key=lambda pair: str(pair[0]))
            if item is not None and item != []
        }

    if isinstance(value, list):
        return [_stable_payload(item) for item in value]

    return value