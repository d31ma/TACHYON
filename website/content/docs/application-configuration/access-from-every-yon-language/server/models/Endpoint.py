from typing import Annotated
import json
import os

class Endpoint:
    port: Annotated[int, setting("${yon.server.port}")] = 0
    redis: Annotated[str, setting("${yon.redis.url:redis://127.0.0.1:6379}")] = ""
