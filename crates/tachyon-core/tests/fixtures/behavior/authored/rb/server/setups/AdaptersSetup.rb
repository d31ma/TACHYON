#[Setup]
#[WhenProfile("test","development")]
#[WhenSetting({"name":"YON_CATALOG_ENABLED","havingValue":"true","matchIfMissing":false})]
class AdaptersSetup
  def self.configure(execution)
    cache = { get: Backend.method(:get), set: Backend.method(:set), delete: Backend.method(:delete) }
    {
      caches: { catalog: cache },
      http: { api: { exchange: Backend.method(:exchange) } },
      entities: { data: { execute: Backend.method(:execute) } },
      authorize: Backend.method(:authorize),
      authorizeEntity: Backend.method(:authorize_entity)
    }
  end
end
