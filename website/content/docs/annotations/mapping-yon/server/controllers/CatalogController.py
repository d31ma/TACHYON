from models import CatalogSummary


@request_route("/catalog")
@public
class CatalogController:
    @get_route
    async def get_catalog(self) -> CatalogSummary:
        signal = BehaviorSignal()
        options = {'partition': 'public-catalog', 'signal': signal}
        context = await Behaviors.create(CatalogSetup, options)
        mapper = Behaviors.construct(ProductConverter, {'behaviors': context})
        product = {'detail': {'id': 7}, 'name': 'Anvil', 'extra': 'dropped'}
        summary = await mapper.to_summary(product)
        context.dispose()
        return CatalogSummary(summary)
