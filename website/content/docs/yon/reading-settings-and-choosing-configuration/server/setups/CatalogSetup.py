from typing import Annotated

@setup
@settings({"prefix":"YON_CATALOG_"})
@when_profile("development")
@when_setting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup:
    pageSize: Annotated[int, min(1), max(100)] = 20
    cacheEnabled: bool = False
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {}
