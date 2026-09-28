@service
@cache_settings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class CatalogService:
    behaviors = None
    reads = 0

    @on_start
    async def open(self) -> None:
        self.behaviors = await Behaviors.create(CatalogSetup, {'partition': 'public-catalog', 'signal': BehaviorSignal()})

    @cached
    async def find(self, sku) -> None:
        self.reads += 1
        return self._entry(sku, self.reads)

    @update_cache({"method":"find"})
    async def write(self, sku, reads) -> None:
        return self._entry(sku, reads)

    @clear_cache({"method":"find"})
    async def invalidate(self, sku) -> None:
        return self._entry(sku, self.reads)

    def _entry(self, sku, reads):
        return {'sku': sku, 'reads': reads}
