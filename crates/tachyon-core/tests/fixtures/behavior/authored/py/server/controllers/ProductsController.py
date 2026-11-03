@request_route("/products")
@public
class ProductsController:
    @get_route
    async def get_products(self):
        signal = BehaviorSignal()
        values = {'YON_CATALOG_SIZE': '25', 'YON_CATALOG_ENABLED': 'true'}
        HandlerChecks.check_configuration(values)
        HandlerChecks.check_value_limits()
        HandlerChecks.check_validation()
        await HandlerChecks.check_inactive_configuration(signal)
        context = await Behaviors.create(AdaptersSetup, {'partition': 'catalog', 'signal': signal, 'profiles': ['test'], 'properties': values})
        result = await HandlerChecks.exercise(context, signal)
        context.dispose()
        return result
