@setup
class OverrideSetup:
    @staticmethod
    def configure(execution: object) -> dict[str, object]:
        return {'http': {'override': OverrideSetup}}

    @staticmethod
    async def exchange(request: object, execution: object) -> dict[str, str]:
        return {'message': 'Custom adapter wins'}
