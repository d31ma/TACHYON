#[allow(dead_code)]
struct ApplicationValues {
    #[Setting({"path":"yon.database.port","type":"integer","default":5432,"min":1,"max":65535})]
    database: u16,
    #[Setting("${yon.database.label:primary}")]
    label: String,
}

