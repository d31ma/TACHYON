@Description("Behavior conformance Backend")
class Backend:
    entries = {}
    calls = 0
    prices = 0
    exchanges = 0
    authorizations = 0
    @staticmethod
    async def authorize(call, execution):
        Backend.authorizations += 1
        if execution.partition != 'catalog':
            return False
        if call['method'] == 'remote':
            return True
        if call['method'] == 'price':
            return len(call['keyArguments']) == 1
        return call['keyArguments'][0]['size'] == 5
    @staticmethod
    async def get(key, execution):
        return Backend.entries.get(key, {'hit': False})
    @staticmethod
    async def set(key, value, execution):
        Backend.entries[key] = {'hit': True, 'value': value, 'expiresAt': execution.expiresAt}
    @staticmethod
    async def delete(key, execution):
        Backend.entries.pop(key, None)
    @staticmethod
    async def exchange(request, execution):
        Backend.exchanges += 1
        assert request['redirect'] == 'error' and request['credentials'] == 'omit'
        assert request['maxResponseBytes'] == 1048576
        return {'path': request['path']}
    @staticmethod
    async def execute(command, execution):
        if command['operation'] != 'query':
            return command
        query = command['query']
        if query['kind'] == 'delete':
            return len(query['where'])
        if not query['where']:
            return [{'id': 'one', 'status': 'active'}]
        if query['where'][0]['field'] == 'id':
            return [{'id': query['where'][0]['value'], 'status': 'active'}]
        assert query['where'][0]['value'] == 'active'
        if query['kind'] == 'count':
            return 1
        if query['kind'] == 'exists':
            return True
        assert query['orderBy'][-1] == {'field': 'id', 'direction': 'asc'}
        return [{'id': 'one', 'status': 'active'}]
    @staticmethod
    async def authorize_entity(command, execution):
        return command['entity'] == 'ProductEntity' and execution.partition == 'catalog'

