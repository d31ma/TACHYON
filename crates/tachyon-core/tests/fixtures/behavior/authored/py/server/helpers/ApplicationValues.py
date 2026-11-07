from typing import Annotated

@Description("Behavior conformance ApplicationValues")
class ApplicationValues:
    port: Annotated[int, setting("${yon.server.port:8080}")] = 0
    size: Annotated[int, setting({"path":"yon.catalog.size","type":"integer","default":5,"min":1,"max":100})] = 0
    redis: Annotated[str, setting("${yon.redis.url:redis://127.0.0.1:6379}")] = ''

