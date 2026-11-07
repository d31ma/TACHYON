@TestSetup
class OrdersTestSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'http': {'orders': OrdersTestSetup}}

    @staticmethod
    async def exchange(request: dict[str, object], execution: object) -> dict[str, str]:
        body = request.get('body')
        return {
            'method': request['method'],
            'path': request['path'],
            'header': next(iter(request['headers'].values())),
            'item': body['item'] if body is not None else 'none',
        }
