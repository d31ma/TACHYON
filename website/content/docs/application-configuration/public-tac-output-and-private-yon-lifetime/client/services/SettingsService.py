from typing import Annotated


@service
class SettingsService:
    application_client: Annotated[ApplicationClient, needs]
    api: str = ''

    async def load(self) -> None:
        application = await self.application_client.load()
        self.api = Behaviors.values(PublicSettings, application)['api']
