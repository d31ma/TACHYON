use crate::models::CacheReport;
use std::sync::Arc;

#[RequestRoute("/catalog")]
#[Public]
struct CatalogController {
    #[Needs]
    catalog: Arc<CatalogService>,
}
impl CatalogController {
    #[GetRoute]
    fn get_catalog(&self) -> CacheReport {
        let sku = BehaviorValue::from("anvil");
        let first = self.catalog.find(sku.clone()).expect("miss");
        let repeat = self.catalog.find(sku.clone()).expect("hit");
        self.catalog.write(sku.clone(), BehaviorValue::Number(99.0)).expect("put");
        let written = self.catalog.find(sku.clone()).expect("written");
        self.catalog.invalidate(sku.clone()).expect("evict");
        let refreshed = self.catalog.find(sku).expect("refreshed");
        CacheReport {
            first: Self::json(first),
            repeat: Self::json(repeat),
            written: Self::json(written),
            refreshed: Self::json(refreshed),
        }
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
