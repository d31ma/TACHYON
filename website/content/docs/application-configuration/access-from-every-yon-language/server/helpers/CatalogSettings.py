from typing import Annotated
import json
import os

@settings({"prefix":"YON_CATALOG_"})
class CatalogSettings:
    pageSize: Annotated[int, min(1), max(100)] = 20
