use crate::models::{Endpoint, Settings};

#[RequestRoute("/settings")]
#[Public]
struct SettingsController {}
impl SettingsController {
    fn new() -> Self { Self {} }
    #[GetRoute]
    fn get_settings(&self) -> Settings {
        let application_json = std::env::var("YON_APPLICATION_JSON").expect("supervisor configuration");
        let application: BehaviorValue = Json::parse(&application_json).expect("validated configuration").into();
        let catalog = Behaviors::properties::<CatalogSetup>(&application).expect("catalog settings");
        let endpoint = Behaviors::values::<Endpoint>(&application).expect("active profile");
        let profile = Self::text(endpoint.get("profile"));
        let mut options = BehaviorOptions::new("public-catalog", BehaviorSignal::new());
        options.profiles.push(profile.clone());
        let options = options.with_application(&application).expect("selection inputs");
        let context = Behaviors::create::<CatalogSetup>(options).expect("active configuration");
        context.dispose();
        Settings { profile, pageSize: Self::number(catalog.get("pageSize")) }
    }

    fn text(value: Option<&BehaviorValue>) -> String {
        let Some(BehaviorValue::String(text)) = value else {
            unreachable!("the bound value is a string")
        };
        text.clone()
    }

    fn number(value: Option<&BehaviorValue>) -> i64 {
        let Some(BehaviorValue::Number(number)) = value else {
            unreachable!("the bound value is a number")
        };
        *number as i64
    }
}
