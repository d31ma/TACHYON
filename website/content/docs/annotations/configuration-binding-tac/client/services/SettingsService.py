@service
class SettingsService:
    title: str = 'Catalog'
    pageSize: int = 20

    @on_start
    async def bind(self) -> None:
        published = self._public_manifest()
        catalog = Behaviors.properties(CatalogSetup, published)
        values = Behaviors.values(PublicValues, published)
        signal = BehaviorSignal()
        context = await Behaviors.create(CatalogSetup, {
            'partition': 'public-catalog',
            'signal': signal,
            'profiles': ['public'],
            'properties': published,
        })
        context.dispose()
        signal.abort()
        self.title = values['title']
        self.pageSize = catalog['pageSize']

    def _public_manifest(self) -> dict[str, object]:
        catalog = {'title': 'Catalog', 'page': {'size': 20}, 'cache': {'enabled': True}}
        return {'tac': {'catalog': catalog}}
