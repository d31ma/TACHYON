@setup
class CatalogSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'caches': {'catalog': MemoryCacheHelper()}, 'authorize': MemoryCacheHelper.allow}
