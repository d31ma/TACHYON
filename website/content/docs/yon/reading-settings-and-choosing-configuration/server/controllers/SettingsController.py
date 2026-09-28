import json
import os

from models import Settings


@request_route("/settings")
@public
class SettingsController:
    @get_route
    async def get_settings(self) -> Settings:
        application = json.loads(os.environ["YON_APPLICATION_JSON"])
        catalog = Behaviors.properties(CatalogSetup, application)
        endpoint = Behaviors.values(Endpoint, application)
        profile = endpoint["profile"]
        signal = BehaviorSignal()
        context = await Behaviors.create(CatalogSetup, {
            "partition": "public-catalog",
            "signal": signal,
            "profiles": [profile],
            "properties": application,
        })
        context.dispose()
        signal.abort()
        return Settings(profile, catalog["pageSize"])
