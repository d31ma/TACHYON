import asyncio
from typing import Annotated
from models import Guide

@service
class LibraryService:
    shelf: Annotated[ShelfHelper, needs]
    total: int = 0
    loaded: str = 'waiting'

    async def load(self, title: str) -> Guide:
        pages = await self.shelf.count(title)
        self.total += pages
        self.loaded = title
        return Guide(title=title, pages=pages)

    async def hold(self) -> int:
        await asyncio.sleep(3600)
        return 0

    async def fail(self) -> int:
        await asyncio.sleep(0)
        raise ValueError('private detail')
