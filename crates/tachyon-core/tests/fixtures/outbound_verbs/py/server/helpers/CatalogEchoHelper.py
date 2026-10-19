@helper
class CatalogEchoHelper:
    @staticmethod
    async def exchange(request: dict[str, object], execution: object) -> dict[str, object]:
        return {'method': request['method'], 'path': request['path'], 'query': request.get('query'), 'headers': request.get('headers'), 'body': request.get('body')}
