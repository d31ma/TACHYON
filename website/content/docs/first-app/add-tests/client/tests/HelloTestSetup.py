@TestSetup
class HelloTestSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'http': {'greeting': HelloTestSetup}}

    @staticmethod
    async def exchange(request: object, execution: object) -> dict[str, str]:
        return {'message': 'Hi'}
