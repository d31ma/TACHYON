use crate::models::Configuration;

#[RequestRoute("/configuration")]
#[Public]
struct ConfigurationController {}
impl ConfigurationController {
    fn new() -> Self { Self {} }
    #[GetRoute]
    fn get_configuration(&self) -> Configuration {
        let application_json = std::env::var("YON_APPLICATION_JSON").expect("supervisor configuration");
        let application: BehaviorValue = Json::parse(&application_json).expect("validated configuration").into();
        let settings = Behaviors::properties::<CatalogSettings>(&application).expect("catalog settings");
        let endpoint = Behaviors::values::<Endpoint>(&application).expect("endpoint settings");
        Configuration { pageSize: Self::number(settings.get("pageSize")), port: Self::number(endpoint.get("port")) }
    }

    fn number(value: Option<&BehaviorValue>) -> i64 {
        let Some(BehaviorValue::Number(number)) = value else {
            unreachable!("the bound value is a number")
        };
        *number as i64
    }
}
