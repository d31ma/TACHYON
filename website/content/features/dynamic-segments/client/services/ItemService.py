@service
class ItemService:
    def id_from(self, path: str) -> str:
        return path.rsplit("/", 1)[-1]
