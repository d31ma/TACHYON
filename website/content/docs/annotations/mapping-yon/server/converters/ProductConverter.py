@converter
class ProductConverter(Protocol):
    @Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})
    async def to_summary(self, product: dict[str, object]) -> None: ...
