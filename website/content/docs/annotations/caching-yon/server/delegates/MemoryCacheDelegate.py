@Delegate
class MemoryCacheDelegate:
    def __init__(self):
        self.entries = {}
    async def get(self, key: str, execution: object) -> dict[str, object]:
        return self.entries.get(key, {'hit': False})
    async def set(self, key: str, value: object, execution: object) -> None:
        self.entries[key] = {'hit': True, 'value': value, 'expiresAt': execution.expiresAt}
    async def delete(self, key: str, execution: object) -> None:
        self.entries.pop(key, None)

    @staticmethod
    async def allow(call: object, execution: object) -> bool:
        return True
