from typing import Annotated

@cache_settings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
@service
class CatalogService:
    PAGE = 'page'
    CALLS = 'calls'

    @cached
    async def read(self, page: Annotated[object, page_defaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]):
        Backend.calls += 1
        return {self.CALLS: Backend.calls, self.PAGE: page}
    @update_cache({"method":"read"})
    async def replace(self, value, page):
        if value == 'fail':
            raise ValueError('failed mutation')
        return {self.CALLS: value, self.PAGE: page}
    @clear_cache({"method":"read"})
    async def remove(self, page):
        return True

