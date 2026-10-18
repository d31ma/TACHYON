import asyncio

@helper
class ShelfHelper:
    async def count(self, title: str) -> int:
        await asyncio.sleep(0.01)
        return len(title)
