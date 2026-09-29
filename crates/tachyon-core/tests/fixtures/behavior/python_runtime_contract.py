import asyncio
import gc
import importlib.util
import pathlib
import time
import warnings


def load_runtime():
    path = pathlib.Path(__file__).parents[3] / 'src/behavior/polyglot/python_runtime.py'
    spec = importlib.util.spec_from_file_location('behavior_runtime', path)
    runtime = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runtime)
    return runtime


class Contract:
    runtime = load_runtime()
    failures = []


async def reject(label, action, code='invalid'):
    try:
        value = action()
        if hasattr(value, '__await__'):
            await value
    except Contract.runtime.BehaviorError as error:
        if error.code != code:
            Contract.failures.append(f'{label}: code {error.code}, expected {code}')
    except BaseException as error:
        Contract.failures.append(f'{label}: leaked {type(error).__name__}')
    else:
        Contract.failures.append(f'{label}: unexpectedly accepted')


def scenario_types(state):
    class Config:
        @staticmethod
        def configure(execution):
            def authorize(call, execution):
                state['authorized'] += 1
                return execution.partition != 'denied'
            def get(key, execution):
                assert 'a' * 64 in key
                return state['hit']
            def put(key, value, execution):
                state['hit'] = {'hit': True, 'value': value, 'expiresAt': execution.expiresAt}
            def delete(key, execution):
                state['hit'] = {'hit': False}
            def exchange(request, execution):
                state['transport'] += 1
                assert request['redirect'] == 'error' and request['credentials'] == 'omit'
                return {'ok': True}
            def entity(command, execution):
                state['commands'].append(command)
                return command['operation']
            return {
                'caches': {'data': {'get': get, 'set': put, 'delete': delete}},
                'authorize': authorize,
                'http': {'api': {'exchange': exchange}},
                'entities': {'store': {'execute': entity}},
            }
    class Service:
        async def read(self, value):
            state['executed'] += 1
            return value
        async def remote(self, descriptor):
            return descriptor
        async def hold(self, page):
            await state['gate'].wait()
            return page
    class Entity:
        pass
    return Config, Service, Entity


def scenario_records(config, service, entity):
    service_methods = {
        'read': {'cacheable': {'keyArgs': [0]}},
        'remote': {'httpExchange': {'client': 'api', 'method': 'GET', 'path': '/products'}},
        'hold': {'pageable': {'argument': 0, 'defaultSize': 20, 'maxSize': 100, 'sort': []}},
    }
    service_cache = {'name': 'data', 'ttlMs': 1000, 'maxBytes': 1024, 'maxEntries': 10}
    return {
        config: {'configuration': True, 'methods': {}},
        service: {'name': 'Service', 'cache': service_cache, 'methods': service_methods},
        entity: {'name': 'Entity', 'entity': {'adapter': 'store', 'id': 'id'}, 'table': 'entities', 'methods': {}},
    }


async def check_cache(state, context, service):
    assert await service.read({'id': 1}) == {'id': 1}
    assert await service.read({'id': 1}) == {'id': 1}
    assert state['authorized'] == 2 and state['executed'] == 1
    await context.evict(service, 'read', [{'id': 1}])
    assert await service.read({'id': 1}) == {'id': 1}
    assert state['executed'] == 2
    state['hit'] = {'hit': True, 'expiresAt': int(time.time() * 1000) + 500}
    await reject('cache missing value', lambda: service.read({}))
    state['hit'] = {'hit': True, 'value': None, 'expiresAt': int(time.time() * 1000) + 100000}
    await reject('cache excessive TTL', lambda: service.read({}))


async def check_http(service):
    await reject('GET body', lambda: service.remote({'body': {}}))
    await reject('Cookie header', lambda: service.remote({'headers': {'Cookie': 'x'}}))
    assert await service.remote({}) == {'ok': True}


async def check_repository(state, context, entity):
    repository = context.repository(entity)
    await reject('save missing id', lambda: repository.save({}))
    await reject('save unsafe id', lambda: repository.save({'id': True}))
    await reject('delete unsafe id', lambda: repository.delete(True))
    assert await repository.save({'id': 'one'}) == 'save'
    assert await repository.find({'page': 2, 'size': 5}) == 'find'
    assert state['commands'][-1]['page']['offset'] == 10


async def check_ownership(factory, context, config, service_type):
    whitespace_options = {'partition': '  ', 'signal': Contract.runtime.BehaviorSignal()}
    await reject('whitespace partition', lambda: factory.create(config, whitespace_options))
    denied = await factory.create(config, {'partition': 'denied', 'signal': Contract.runtime.BehaviorSignal()})
    outsider = service_type()
    outsider.behaviors = denied
    await reject('cross owner eviction', lambda: context.evict(outsider, 'read', [{}]))
    await reject('denied read', lambda: outsider.read({}), 'unauthorized')


async def check_capacity(state, context, service):
    state['gate'] = asyncio.Event()
    pending = [asyncio.create_task(service.hold({})) for _ in range(64)]
    for _ in range(10):
        if context.active == 64:
            break
        await asyncio.sleep(0)
    assert context.active == 64
    await reject('concurrency capacity', lambda: service.hold({}))
    state['gate'].set()
    await asyncio.gather(*pending)
    assert context.active == 0


async def check_deadline(state, factory, config, service_type):
    state['gate'] = asyncio.Event()
    timed_options = {'partition': 'tenant', 'signal': Contract.runtime.BehaviorSignal(), 'timeoutMs': 20}
    timed = await factory.create(config, timed_options)
    timed_service = service_type()
    timed_service.behaviors = timed
    await reject('async deadline', lambda: timed_service.hold({}), 'timeout')
    assert timed.active == 0


async def check_cancellation(context, service, signal):
    pending = asyncio.create_task(service.hold({}))
    await asyncio.sleep(0)
    signal.abort()
    await reject('pending owner cancellation', lambda: pending, 'cancelled')
    assert context.active == 0
    await reject('aborted read', lambda: service.read({}), 'cancelled')


async def check_prechecked_coroutine(context):
    execution = Contract.runtime._BehaviorExecution(context)
    async def unused():
        return None
    with warnings.catch_warnings(record=True) as captured:
        warnings.simplefilter('always')
        await reject('prechecked coroutine', lambda: Contract.runtime._behavior_await(unused(), execution), 'cancelled')
        gc.collect()
        if captured:
            Contract.failures.append('prechecked coroutine: leaked an unawaited coroutine warning')


async def scenario():
    state = {'hit': {'hit': False}, 'authorized': 0, 'executed': 0, 'transport': 0, 'commands': []}
    config, service_type, entity = scenario_types(state)
    records = scenario_records(config, service_type, entity)
    factory = Contract.runtime._BehaviorFactory(records, 'yon', 'a' * 64)
    signal = Contract.runtime.BehaviorSignal()
    context = await factory.create(config, {'partition': 'tenant', 'signal': signal})
    service = service_type()
    service.behaviors = context
    await check_cache(state, context, service)
    await check_http(service)
    await check_repository(state, context, entity)
    await check_ownership(factory, context, config, service_type)
    await check_capacity(state, context, service)
    await check_deadline(state, factory, config, service_type)
    await check_cancellation(context, service, signal)
    await check_prechecked_coroutine(context)


def resilience_types(state):
    class Config:
        @staticmethod
        def configure(execution):
            return {}
    class Orders:
        async def flaky(self, value):
            state['attempts'] += 1
            if state['attempts'] < 3:
                raise RuntimeError('transient')
            return value
        async def broken(self, value):
            state['attempts'] += 1
            raise RuntimeError('permanent')
        async def hold(self, value):
            state['active'] += 1
            state['peak'] = max(state['peak'], state['active'])
            await state['gate'].wait()
            state['active'] -= 1
            return value
        async def refuse(self, value):
            await state['gate'].wait()
            return value
    return Config, Orders


def resilience_records(config, orders):
    retryable = {'maxRetries': 3, 'delayMs': 0, 'multiplier': 1, 'maxDelayMs': 100, 'jitterMs': 0}
    orders_record = {'name': 'Orders', 'configuration': False, 'methods': {
        'flaky': {'retryable': retryable},
        'broken': {'retryable': retryable},
        'hold': {'concurrencyLimit': {'limit': 2, 'policy': 'block'}},
        'refuse': {'concurrencyLimit': {'limit': 1, 'policy': 'reject'}},
    }}
    return {orders: orders_record, config: {'name': 'Config', 'configuration': True, 'methods': {}}}


async def check_retries(state, orders):
    assert await orders.flaky(7) == 7
    assert state['attempts'] == 3, state['attempts']
    state['attempts'] = 0
    try:
        await orders.broken(1)
        Contract.failures.append('exhausted retries: unexpectedly accepted')
    except RuntimeError:
        pass
    assert state['attempts'] == 4, state['attempts']


async def check_blocking_limit(state, orders):
    pending = [asyncio.create_task(orders.hold(index)) for index in range(4)]
    for _ in range(10):
        await asyncio.sleep(0)
    assert state['peak'] == 2, state['peak']
    state['gate'].set()
    assert await asyncio.gather(*pending) == [0, 1, 2, 3]
    assert state['peak'] == 2, state['peak']


async def check_rejecting_limit(state, orders, context):
    state['gate'] = asyncio.Event()
    held = asyncio.create_task(orders.refuse(1))
    for _ in range(10):
        await asyncio.sleep(0)
    await reject('method concurrency limit', lambda: orders.refuse(2), 'capacity')
    state['gate'].set()
    assert await held == 1
    assert not context._slots, context._slots


async def resilience():
    state = {'attempts': 0, 'peak': 0, 'active': 0, 'gate': asyncio.Event()}
    config, orders_type = resilience_types(state)
    records = resilience_records(config, orders_type)
    factory = Contract.runtime._BehaviorFactory(records, 'yon', 'a' * 64)
    context = await factory.create(config, {'partition': 'tenant', 'signal': Contract.runtime.BehaviorSignal()})
    orders = orders_type()
    orders.behaviors = context
    await check_retries(state, orders)
    await check_blocking_limit(state, orders)
    await check_rejecting_limit(state, orders, context)


def transaction_types(log, state):
    class Config:
        @staticmethod
        def configure(execution):
            async def execute(command, execution):
                if command['operation'] != 'transaction':
                    log.append('%s@%s' % (command['operation'], command.get('transaction', 'none')))
                    return {'id': 'one'}
                entry = command['action']
                if 'transaction' in command:
                    entry += ':' + command['transaction']
                log.append(entry)
                if command['action'] == 'begin':
                    state['sequence'] += 1
                    return {'transaction': 't%d' % state['sequence']}
                return {'ok': True}
            return {'entities': {'data': {'execute': execute}}}
    class Order:
        pass
    class Orders:
        async def place(self, fail):
            await self.behaviors.repository(Order).save({'id': 'one'})
            if fail:
                raise RuntimeError('domain rejected the order')
            return 'placed'
    return Config, Order, Orders


async def transactions():
    log = []
    state = {'sequence': 0}
    config, order, orders_type = transaction_types(log, state)
    transactional = {'transactional': {'adapter': 'data'}}
    records = {
        config: {'name': 'Config', 'configuration': True, 'methods': {}},
        order: {'name': 'Order', 'configuration': False, 'entity': {'adapter': 'data', 'id': 'id'}, 'table': 'orders', 'methods': {}},
        orders_type: {'name': 'Orders', 'configuration': False, 'methods': {'place': transactional}},
    }
    factory = Contract.runtime._BehaviorFactory(records, 'yon', 'a' * 64)
    context = await factory.create(config, {'partition': 'tenant', 'signal': Contract.runtime.BehaviorSignal()})
    orders = orders_type()
    orders.behaviors = context
    assert await orders.place(False) == 'placed'
    assert log == ['begin', 'save@t1', 'commit:t1'], log
    log.clear()
    try:
        await orders.place(True)
        Contract.failures.append('a failing transactional body was accepted')
    except RuntimeError:
        pass
    assert log == ['begin', 'save@t2', 'rollback:t2'], log
    log.clear()
    await context.repository(order).save({'id': 'two'})
    assert log == ['save@none'], log


def ledger_types(log, state):
    class LedgerSetup:
        @staticmethod
        def configure(execution):
            async def execute(command, execution):
                if command['operation'] == 'transaction':
                    log.append(command['action'] + (':' + command['transaction'] if 'transaction' in command else ''))
                    if command['action'] != 'begin':
                        return {'ok': True}
                    state['sequence'] += 1
                    return {'transaction': 't%d' % state['sequence']}
                log.append('%s@%s' % (command['operation'], command.get('transaction', 'none')))
                if command['value']['id'] == 'broken':
                    raise RuntimeError('the second write failed')
                return {'id': command['value']['id']}
            return {'entities': {'ledger': {'execute': execute}, 'audit': {'execute': execute}}}
    class Entry:
        pass
    class Note:
        pass
    class LedgerService:
        async def settle(self, second):
            await self.entries.repository(Entry).save({'id': 'first'})
            await self.entries.repository(Note).save({'id': 'note'})
            await self.entries.repository(Entry).save({'id': second})
            return 'settled'
    return LedgerSetup, Entry, Note, LedgerService


async def joined_transactions():
    log = []
    state = {'sequence': 0}
    setup, entry, note, service_type = ledger_types(log, state)
    setup_record = {'name': 'LedgerSetup', 'configuration': True, 'methods': {}}
    store_runtime = load_runtime()
    stores = store_runtime._BehaviorFactory({
        setup: setup_record,
        entry: {'name': 'Entry', 'configuration': False, 'entity': {'adapter': 'ledger', 'id': 'id'}, 'table': 'entries', 'methods': {}},
        note: {'name': 'Note', 'configuration': False, 'entity': {'adapter': 'audit', 'id': 'id'}, 'table': 'notes', 'methods': {}},
    }, 'yon', 'b' * 64)
    services = Contract.runtime._BehaviorFactory({
        setup: setup_record,
        service_type: {'name': 'LedgerService', 'configuration': False, 'methods': {'settle': {'transactional': {'adapter': 'ledger'}}}},
    }, 'yon', 'a' * 64)
    ledger = service_type()
    ledger.behaviors = await services.create(setup, {'partition': 'ledger', 'signal': Contract.runtime.BehaviorSignal()})
    ledger.entries = await stores.create(setup, {'partition': 'entries', 'signal': store_runtime.BehaviorSignal()})
    assert await ledger.settle('second') == 'settled'
    assert log == ['begin', 'save@t1', 'save@none', 'save@t1', 'commit:t1'], log
    log.clear()
    try:
        await ledger.settle('broken')
        Contract.failures.append('a failed second write was accepted')
    except Exception:
        pass
    assert log == ['begin', 'save@t2', 'save@none', 'save@t2', 'rollback:t2'], log


async def rest_client():
    seen = []
    class Config:
        @staticmethod
        def configure(execution):
            async def exchange(request, execution):
                assert request['redirect'] == 'error' and request['credentials'] == 'omit'
                seen.append('%s %s %s' % (request['method'], request['path'], sorted(request['query'].items())))
                return {'ok': True, 'body': request.get('body')}
            return {'http': {'catalog': {'exchange': exchange}}}
    records = {Config: {'name': 'Config', 'configuration': True, 'methods': {}}}
    factory = Contract.runtime._BehaviorFactory(records, 'yon', 'a' * 64)
    context = await factory.create(Config, {'partition': 'tenant', 'signal': Contract.runtime.BehaviorSignal()})
    client = context.rest_client('catalog')
    read = await client.get('/products').query({'page': 2}).header('X-Test', 'present').retrieve()
    assert read == {'body': None, 'ok': True}
    written = await client.post('/products').body({'sku': 'anvil'}).retrieve()
    assert written['body'] == {'sku': 'anvil'}, written
    assert seen == ["GET /products [('page', 2)]", "POST /products []"], seen
    await reject('RestClient query path', lambda: client.get('/products?page=1'))
    await reject('RestClient traversal', lambda: client.get('/../secrets'))
    await reject('RestClient read body', lambda: client.get('/products').body({}))
    await reject('RestClient adapter name', lambda: context.rest_client('not an identifier'))
    await reject('RestClient transport header', lambda: client.get('/products').header('Host', 'attacker.example').retrieve())


async def main():
    runtime = Contract.runtime
    await reject('huge integer', lambda: runtime._behavior_copy(10 ** 1000))
    await reject('invalid UTF-8 scalar', lambda: runtime._behavior_copy('\ud800'))
    await reject('nonfinite value', lambda: runtime._behavior_copy(float('nan')))
    await reject('JSON bytes', lambda: runtime._behavior_copy('x' * 1048577))
    await reject('boolean offset', lambda: runtime._behavior_page({'offset': False}, {'defaultSize': 20, 'maxSize': 100, 'sort': []}))
    await scenario()
    await resilience()
    await transactions()
    await joined_transactions()
    await rest_client()
    assert not Contract.failures, '\n'.join(Contract.failures)
    print('Python behavior boundary contracts passed')


asyncio.run(main())
