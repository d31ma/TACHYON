use super::{Module, Type};

pub(super) struct TestClients;

impl TestClients {
    const EXCHANGE: &'static str = "httpExchange";

    pub(super) fn is_client(module: &Module, name: &str) -> bool {
        module
            .types
            .iter()
            .any(|declared| Self::declares_client(declared, name))
    }

    pub(super) fn sends_requests(declared: &Type) -> bool {
        declared
            .methods
            .iter()
            .any(|method| method.metadata.contains_key(Self::EXCHANGE))
    }

    fn declares_client(declared: &Type, name: &str) -> bool {
        [declared.name == name, Self::sends_requests(declared)] == [true; 2]
    }
}
