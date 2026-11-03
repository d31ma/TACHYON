use super::Choice;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RealmChoice {
    Both,
    Client,
    Server,
}

impl RealmChoice {
    const NAMES: [&'static str; 3] = ["both", "client,tac", "server,yon"];
    const LABELS: [&'static str; 3] = [
        "Both: pages (Tac) and a server (Yon)",
        "Client: pages only (Tac)",
        "Server: a server only (Yon)",
    ];

    #[must_use]
    pub fn has_client(self) -> bool {
        self != Self::Server
    }

    #[must_use]
    pub fn has_server(self) -> bool {
        self != Self::Client
    }
}

impl Choice for RealmChoice {
    const ALL: &'static [Self] = &[Self::Both, Self::Client, Self::Server];
    const QUESTION: &'static str = "What should the app have?";

    fn names(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn label(self) -> &'static str {
        Self::LABELS[self as usize]
    }
}
