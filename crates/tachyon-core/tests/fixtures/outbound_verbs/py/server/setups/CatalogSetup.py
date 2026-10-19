@setup
class CatalogSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'http': {'catalog': CatalogEchoHelper}}
