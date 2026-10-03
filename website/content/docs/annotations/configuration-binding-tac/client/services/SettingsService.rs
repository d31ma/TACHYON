#[Service]
#[allow(non_snake_case)]
struct SettingsService {
    title: String,
    pageSize: i64,
}
impl SettingsService {
    const TITLE: &str = "title";
    const DEFAULT_TITLE: &str = "Catalog";

    fn new() -> Self {
        let public_catalog = BehaviorValue::object([
            (Self::TITLE, BehaviorValue::String(Self::DEFAULT_TITLE.into())),
            ("page", BehaviorValue::object([("size", BehaviorValue::Number(20.0))])),
            ("cache", BehaviorValue::object([("enabled", BehaviorValue::Bool(true))])),
        ]);
        let public_settings = BehaviorValue::object([("catalog", public_catalog)]);
        let published = BehaviorValue::object([("tac", public_settings)]);
        let catalog = Behaviors::properties::<CatalogSetup>(&published).expect("public catalog settings");
        let values = Behaviors::values::<PublicValues>(&published).expect("public values");
        let mut options = BehaviorOptions::new("public-catalog", BehaviorSignal::new());
        options.profiles.push("public".into());
        let options = options.with_application(&published).expect("public selection inputs");
        let context = Behaviors::create::<CatalogSetup>(options).expect("active configuration");
        context.dispose();
        Self { title: Self::title_of(values.get(Self::TITLE)), pageSize: Self::page_size_of(catalog.get("pageSize")) }
    }

    fn title_of(value: Option<&BehaviorValue>) -> String {
        match value {
            Some(BehaviorValue::String(text)) => text.clone(),
            _ => Self::DEFAULT_TITLE.into(),
        }
    }

    fn page_size_of(value: Option<&BehaviorValue>) -> i64 {
        match value {
            Some(BehaviorValue::Number(size)) => *size as i64,
            _ => 20,
        }
    }
}
