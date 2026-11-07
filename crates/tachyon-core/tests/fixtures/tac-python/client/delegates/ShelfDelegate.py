import asyncio

@delegate
class ShelfDelegate:
    async def count(self, title: str) -> int:
        await asyncio.sleep(0.01)
        return len(title)
