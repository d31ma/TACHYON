from typing import Annotated

@cache_settings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
@http_client({"adapter": "api"})
class CatalogClient:
    PAGE = 'page'
    CALLS = 'calls'

    @cached
    async def read(self, page: Annotated[object, page_defaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]):
        Backend.calls += 1
        return {self.CALLS: Backend.calls, self.PAGE: page}
    @cached
    @get_request("/products")
    async def remote(self):
        return {'query': {self.PAGE: 1}}
    @get_request("/products")
    async def search(self, page: Annotated[object, request_param("page")]):
        pass
    @get_request("/products/{id}")
    async def find(self, id: Annotated[object, path_variable]):
        pass
    @update_cache({"method":"read"})
    async def replace(self, value, page):
        if value == 'fail':
            raise ValueError('failed mutation')
        return {self.CALLS: value, self.PAGE: page}
    @clear_cache({"method":"read"})
    async def remove(self, page):
        return True

