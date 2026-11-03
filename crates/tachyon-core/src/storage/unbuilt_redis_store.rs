use serde_json::Value;

use super::failure;
use crate::Failure;

pub(super) enum UnbuiltRedisStore {}

impl UnbuiltRedisStore {
    pub(super) fn connect(_settings: &Value) -> std::future::Ready<Result<Self, Failure>> {
        std::future::ready(Err(failure(
            "This ty was built without Redis support; rebuild it with `--features storage-redis` (release builds include it).",
        )))
    }

    #[allow(clippy::unused_self)]
    pub(super) fn execute(&self, _command: &Value) -> std::future::Ready<Result<Value, Failure>> {
        std::future::ready(Err(failure("Redis resource is closed.")))
    }

    #[allow(clippy::unused_self)]
    pub(super) fn close(&self) -> std::future::Ready<()> {
        std::future::ready(())
    }
}
