@Description("Behavior conformance HandlerChecks")
class HandlerChecks:
    @staticmethod
    def behavior_error(action, message):
        try:
            action()
        except BehaviorError as error:
            return error
        raise AssertionError(message)

    @staticmethod
    async def behavior_error_async(action, message):
        try:
            await action()
        except BehaviorError as error:
            return error
        raise AssertionError(message)

    @staticmethod
    def check_configuration(values):
        assert Behaviors.properties(Settings, values) == {'size': 25, 'enabled': True}
        application = {'yon': {'server': {'host': '127.0.0.1', 'port': 9000}, 'catalog': {'size': 25}}}
        nested_settings = {'yon': {'catalog': {'size': 25, 'enabled': True}}}
        assert Behaviors.properties(Settings, nested_settings) == {'size': 25, 'enabled': True}
        assert Behaviors.values(ApplicationValues, application) == {'port': 9000, 'size': 25, 'redis': 'redis://127.0.0.1:6379'}

    @staticmethod
    def check_value_limits():
        for text in ('x' * 4096, 'é' * 2048):
            assert Behaviors.values(ApplicationValues, {'yon': {'redis': {'url': text}}})['redis'] == text
        for text in ('x' * 4097, 'é' * 2049):
            error = HandlerChecks.behavior_error(
                lambda: Behaviors.values(ApplicationValues, {'yon': {'redis': {'url': text}}}),
                'oversized selected value must fail',
            )
            assert error.code == 'configuration-properties' and text not in str(error)
        error = HandlerChecks.behavior_error(
            lambda: Behaviors.values(ApplicationValues, {'yon': {'datasource': {'password': 'x' * 4097}}}),
            'oversized scalar accepted',
        )
        assert error.code == 'configuration-properties'
        error = HandlerChecks.behavior_error(
            lambda: Behaviors.values(ApplicationValues, {'server': {'port': '9000'}}),
            'a string port must fail',
        )
        assert error.code == 'configuration-properties'

    @staticmethod
    def check_validation():
        assert ValidationService().accept({'name': 'éé', 'price': 20}) == {'name': 'éé', 'price': 20}
        error = HandlerChecks.behavior_error(
            lambda: Behaviors.validate(ProductInput, {'name': '\u0085\u00a0'}),
            'validation must fail',
        )
        assert error.code == 'validation' and error.path == 'name.notBlank'

    @staticmethod
    async def check_inactive_configuration(signal):
        error = await HandlerChecks.behavior_error_async(
            lambda: Behaviors.create(AdaptersSetup, {'partition': 'catalog', 'signal': signal}),
            'configuration must be inactive',
        )
        assert error.code == 'inactive-configuration'

    @staticmethod
    async def exercise_cache(catalog, context, page):
        first = await catalog.read(page)
        cached = await catalog.read(page)
        await context.evict(catalog, 'read', [page])
        fresh = await catalog.read(page)
        await catalog.replace(99, page)
        assert (await catalog.read(page))['calls'] == 99
        try:
            await catalog.replace('fail', page)
            raise AssertionError('mutation must fail')
        except ValueError:
            pass
        assert (await catalog.read(page))['calls'] == 99
        await catalog.remove(page)
        assert (await catalog.read(page))['calls'] == 3
        return first, cached, fresh

    @staticmethod
    async def check_resilience(context):
        resilient = Behaviors.construct(Resilient, {'behaviors': context})
        assert await resilient.flaky() == 3 and Resilient.attempts == 3
        assert await resilient.bounded('ok') == 'ok'

    @staticmethod
    async def check_queries(context):
        products = Behaviors.construct(ProductsStore, {'behaviors': context})
        assert await products.find_by_status('active') == [{'id': 'one', 'status': 'active'}]
        assert await products.countByStatus('active') == 1
        assert await products.available('active') is True

    @staticmethod
    async def check_mapping(context):
        summaries = Behaviors.construct(ProductSummaries, {'behaviors': context})
        product = {'detail': {'id': 7}, 'name': 'Anvil', 'extra': 'dropped'}
        assert await summaries.to_summary(product) == {
            'productId': 7, 'title': 'Anvil', 'absent': None, 'kind': 'product',
        }
        error = await HandlerChecks.behavior_error_async(
            lambda: summaries.to_summary(['not an object']),
            'a mapping reads one JSON object argument',
        )
        assert error.code == 'invalid'

    @staticmethod
    async def is_cancelled(catalog):
        try:
            await catalog.remote()
        except BehaviorError as error:
            return error.code == 'cancelled'
        return False

    @staticmethod
    async def exercise(context, signal):
        catalog = CatalogService()
        catalog.behaviors = context
        client = CatalogClient()
        client.behaviors = context
        page = {'page': 2, 'size': 5}
        first, cached, fresh = await HandlerChecks.exercise_cache(catalog, context, page)
        remote = await client.remote()
        assert await client.remote() == remote and Backend.exchanges == 1
        assert await client.find(7) == {'path': '/products/7'}
        await HandlerChecks.check_resilience(context)
        repository = context.repository(ProductEntity)
        saved = await repository.save({'id': 'one'})
        found = await repository.find()
        deleted = await repository.delete('one')
        await HandlerChecks.check_queries(context)
        await HandlerChecks.check_mapping(context)
        signal.abort()
        cancelled = await HandlerChecks.is_cancelled(client)
        return {
            'first': first,
            'cached': cached,
            'fresh': fresh,
            'remote': remote,
            'saved': saved,
            'found': found,
            'deleted': deleted,
            'cancelled': cancelled,
            'authorizations': Backend.authorizations,
        }

