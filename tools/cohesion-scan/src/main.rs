use cohesion_scan::{Report, Scan};
use std::env;
use std::error::Error;
use std::path::PathBuf;

struct Command;

impl Command {
    const JSON_FLAG: &'static str = "--json";

    fn run() -> Result<(), Box<dyn Error>> {
        let arguments: Vec<String> = env::args().skip(1).collect();
        let root = arguments
            .iter()
            .find(|argument| !argument.starts_with("--"))
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        let report = Scan::repository(&root)?;
        println!("{}", Self::render(&report, &arguments)?);
        Ok(())
    }

    fn render(report: &Report, arguments: &[String]) -> Result<String, serde_json::Error> {
        if arguments.iter().any(|argument| argument == Self::JSON_FLAG) {
            return report.json();
        }
        Ok(report.table())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    Command::run()
}
