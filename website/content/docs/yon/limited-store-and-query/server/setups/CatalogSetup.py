@setup
class CatalogSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'entities': {'catalog': CatalogAdapterDelegate}, 'authorizeEntity': CatalogSetup.authorize}

    @staticmethod
    async def authorize(command: dict[str, object], execution: object) -> bool:
        return command['entity'] == 'ProductEntity'
