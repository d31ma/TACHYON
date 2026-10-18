from typing import Annotated


@setup
@settings({"prefix":"TAC_CATALOG_"})
@when_profile("public")
@when_setting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
class CatalogSetup:
    pageSize: Annotated[int, min(1), max(100)] = 20
    cacheEnabled: bool = False
    title: str = 'Catalog'

    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {}
