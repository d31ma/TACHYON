@setup
class CatalogSetup:
    @staticmethod
    def configure(execution):
        return {'caches': {'catalog': MemoryCacheHelper()}, 'authorize': MemoryCacheHelper.allow}
