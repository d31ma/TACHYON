from typing import Annotated


class PublicSettings:
    api: Annotated[str, setting("${tac.api.url:/api}")] = ''
