@TestSetup
class OrdersTestSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'http': {'orders': OrdersTestSetup}}

    @staticmethod
    async def exchange(request: dict[str, object], execution: object) -> dict[str, str]:
        if request['path'] == '/api/claims/a-7':
            raise BehaviorError('HTTP_STATUS', 'private fake text', None, 409, {'reason': 'taken', 'holder': 'south'})
        body = request.get('body')
        return {
            'method': request['method'],
            'path': request['path'],
            'header': next(iter(request['headers'].values())),
            'item': body['item'] if body is not None else 'none',
        }
