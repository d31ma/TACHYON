import json
import os

from models import CatalogSettings, Configuration, Endpoint


@request_route("/configuration")
@public
class ConfigurationController:
    @get_route
    def get_configuration(self) -> Configuration:
        application = json.loads(os.environ["YON_APPLICATION_JSON"])
        settings = Behaviors.properties(CatalogSettings, application)
        endpoint = Behaviors.values(Endpoint, application)
        return Configuration(settings["pageSize"], endpoint["port"])
