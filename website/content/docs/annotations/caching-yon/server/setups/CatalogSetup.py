@setup
class CatalogSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'caches': {'catalog': MemoryCacheDelegate()}, 'authorize': MemoryCacheDelegate.allow}
