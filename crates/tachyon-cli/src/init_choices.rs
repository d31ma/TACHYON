use clap::Args;
use tachyon_core::ChoiceAnswers;
use tachyon_core::tool_setup::Consent;

#[derive(Clone, Debug, Args)]
pub struct InitChoices {
    #[arg(long, help = "Parts to create: client, server or both (default both)")]
    realm: Option<String>,
    #[arg(
        long,
        help = "Tac (client) languages, separated by commas: js, ts, dart, rs, cs, fs, go, rb, swift, kt (default js)"
    )]
    tac: Option<String>,
    #[arg(
        long,
        help = "Yon (server) languages, separated by commas: js, ts, py, go, java, kt, cs, fs, php, rb, rs, dart (default js)"
    )]
    yon: Option<String>,
    #[arg(
        long = "target",
        visible_alias = "targets",
        help = "Targets, separated by commas: web, macos, windows, linux, ios, android (default web)"
    )]
    targets: Option<String>,
    #[arg(
        long = "features",
        visible_alias = "feature",
        help = "Examples, separated by commas: storage, sockets, topics, streams, http-client, error-handlers"
    )]
    features: Option<String>,
    #[arg(
        long,
        short = 'y',
        help = "Use the default for every question that has no option, and install every missing tool"
    )]
    pub(crate) yes: bool,
    #[arg(long, help = "Ask the questions even when the input is not a terminal")]
    pub(crate) interactive: bool,
    #[arg(
        long,
        help = "Do not check or install the tools that the project needs"
    )]
    pub(crate) skip_tools: bool,
}

impl InitChoices {
    pub fn should_ask(&self, terminal: bool) -> bool {
        self.asks(terminal)
    }

    pub fn consent(&self, terminal: bool) -> Consent {
        Consent::from_options(self.yes, self.can_ask(terminal))
    }

    fn can_ask(&self, terminal: bool) -> bool {
        terminal || self.interactive
    }

    fn asks(&self, terminal: bool) -> bool {
        let quiet_terminal = [terminal, self.answers().is_empty(), !self.yes] == [true, true, true];
        self.interactive || quiet_terminal
    }

    pub fn answers(&self) -> ChoiceAnswers {
        ChoiceAnswers {
            realm: self.realm.clone(),
            tac: self.tac.clone(),
            yon: self.yon.clone(),
            targets: self.targets.clone(),
            features: self.features.clone(),
        }
    }
}
