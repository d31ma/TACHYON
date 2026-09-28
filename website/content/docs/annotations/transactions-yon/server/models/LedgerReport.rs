use crate::Json;

pub struct LedgerReport {
    pub settled: Json,
    pub commands: Json,
}
