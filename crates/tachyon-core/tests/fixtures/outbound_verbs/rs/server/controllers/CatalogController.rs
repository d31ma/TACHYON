#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {
  #[Needs]
  catalog: std::sync::Arc<CatalogClient>,
}
impl CatalogController {
    fn product(sku: &str, level: f64) -> BehaviorValue {
        BehaviorValue::object([("sku", BehaviorValue::from(sku)), ("level", BehaviorValue::Number(level))])
    }

    fn sent(&self) -> BehaviorResult<BehaviorValue> {
        Ok(BehaviorValue::object([
            ("found", self.catalog.find("anvil".into(), "full".into(), "acme".into())?),
            ("saved", self.catalog.save(Self::product("anvil", 3.0))?),
            ("replaced", self.catalog.replace(BehaviorValue::Number(5.0), Self::product("anvil", 4.0))?),
            ("renamed", self.catalog.rename(BehaviorValue::Number(6.0), Self::product("hammer", 1.0))?),
            ("removed", self.catalog.remove(BehaviorValue::Number(7.0))?),
        ]))
    }

    #[GetRoute]
    fn get_catalog(&self) -> Json {
        let sent = self.sent().and_then(|value| value.to_json(65536));
        sent.ok().and_then(|text| Json::parse(&text).ok()).unwrap_or(Json::Null)
    }
}
