#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WebEventSender {
    Page,
    Grant(&'static str),
    Socket,
}

impl WebEventSender {
    pub(crate) fn reason(self) -> String {
        self.sending_rule()
    }

    fn sending_rule(self) -> String {
        match self {
            Self::Page => String::from("the page always sends it"),
            Self::Grant(api) => {
                format!("tac.browserApis in client/manifest.json grants \"{api}\", which sends it")
            }
            Self::Socket => String::from(
                "the service holds a socket client, and the event bus sends it to that service",
            ),
        }
    }
}
