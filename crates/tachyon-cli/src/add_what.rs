use crate::add_options::AddOptions;
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum AddWhat {
    #[command(
        about = "Add a Tac or Yon language with an example and its test, for example: ty add language server py"
    )]
    Language {
        #[arg(help = "client (Tac) or server (Yon)")]
        realm: String,
        #[arg(
            help = "Language: js, ts, dart, rs, cs, fs, go, rb, lua, cpp, swift, kt, java for client; js, ts, py, go, java, kt, cs, php, rb, lua, rs, dart for server"
        )]
        language: String,
        #[command(flatten)]
        options: AddOptions,
    },
    #[command(about = "Add a build target, for example: ty add target ios")]
    Target {
        #[arg(help = "Target: web, macos, windows, linux, ios, android")]
        target: String,
        #[command(flatten)]
        options: AddOptions,
    },
    #[command(about = "Add an example with its tests, for example: ty add feature sockets")]
    Feature {
        #[arg(help = "Example: storage, sockets, topics, streams, http-client, error-handlers")]
        feature: String,
        #[command(flatten)]
        options: AddOptions,
    },
}
