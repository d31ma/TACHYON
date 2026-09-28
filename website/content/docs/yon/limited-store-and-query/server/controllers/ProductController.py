from models import ProductReport


@request_route("/products")
@public
class ProductController:
    @get_route
    async def get_product(self) -> ProductReport:
        signal = BehaviorSignal()
        context = await Behaviors.create(CatalogSetup, {'partition': 'public-catalog', 'signal': signal})
        products = Behaviors.construct(ProductStore, {'behaviors': context})
        active_status = 'active'
        active = await products.find_by_status(active_status)
        active_count = await products.count_by_status(active_status)
        page = products.bounded({'size': 5})
        others = await products.excluding(active_status, page)
        context.dispose()
        return ProductReport(active, active_count, others)
