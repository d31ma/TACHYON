use crate::Json;

pub struct CacheReport {
    pub first: Json,
    pub repeat: Json,
    pub written: Json,
    pub refreshed: Json,
}
