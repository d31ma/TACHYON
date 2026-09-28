from typing import Annotated

@settings({"prefix":"YON_CATALOG_"})
class Settings:
    size: Annotated[int, min(1), max(100)] = 20
    enabled: bool = False

