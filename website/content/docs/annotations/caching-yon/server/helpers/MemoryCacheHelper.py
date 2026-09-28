@Helper
class MemoryCacheHelper:
    def __init__(self):
        self.entries = {}
    async def get(self, key, execution) -> None:
        return self.entries.get(key, {'hit': False})
    async def set(self, key, value, execution) -> None:
        self.entries[key] = {'hit': True, 'value': value, 'expiresAt': execution.expiresAt}
    async def delete(self, key, execution) -> None:
        self.entries.pop(key, None)

    @staticmethod
    async def allow(call, execution) -> None:
        return True
