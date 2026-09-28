@setup
@when_profile("test","development")
@when_setting({"name":"YON_CATALOG_ENABLED","havingValue":"true","matchIfMissing":false})
class AdaptersSetup:
    @staticmethod
    def configure(execution):
        return {
            'caches': {'catalog': Backend},
            'http': {'api': Backend},
            'entities': {'data': Backend},
            'authorize': Backend.authorize,
            'authorizeEntity': Backend.authorize_entity,
        }

