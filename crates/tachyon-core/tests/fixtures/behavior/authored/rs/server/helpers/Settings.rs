#[Settings({"prefix":"YON_DATABASE_","defaults":{"port":5432,"enabled":false,"ratio":0.5}})]
struct Settings {
    host: String,
    #[Min(1)]
    #[Max(65535)]
    port: i64,
    enabled: bool,
    #[Min(0)]
    #[Max(1)]
    ratio: f64,
}

