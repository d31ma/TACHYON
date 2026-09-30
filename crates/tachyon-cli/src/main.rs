mod add_command;
mod add_options;
mod add_what;
mod advice_report;
mod coverage_gate;
mod init_choices;
mod init_command;
mod resilient_serve;
mod restart;
mod shutdown;
mod start_step;
mod test_output;
mod tool_step;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::hash::{Hash as _, Hasher as _};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod, NativeTarget,
};
use tachyon_core::{
    ApiBaselineCheck, ApplicationConfig, BuildOptions, ComposedTypes, DevServer, DevServerOptions,
    EnvironmentPolicy, Failure, HandlerCancellation, HandlerRuntimePrograms, HandlerSource,
    HandlerSupervisor, HandlerSupervisorOptions, MigrationAnalysis, MigrationFix,
    NativeBuildOptions, NativeCompiler, PreviewServer, PreviewServerOptions, WebCompiler,
    method_coverage::RecordedCalls, native_target_directory,
};
use tachyon_diagnostics::{Diagnostic, DiagnosticCode, Severity};

use add_command::AddCommand;
use add_what::AddWhat;
use advice_report::AdviceReport;
use coverage_gate::CoverageGate;
use init_choices::InitChoices;
use init_command::InitCommand;
use resilient_serve::ResilientServe;
use shutdown::{LongRunningCommand, ShutdownSignals};
use test_output::TestOutput;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum DiagnosticFormat {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BuildTarget {
    #[value(alias = "browser")]
    Web,
    #[value(aliases = ["mac", "darwin", "osx"])]
    Macos,
    Ios,
    Linux,
    #[value(aliases = ["win", "win32"])]
    Windows,
    Android,
    All,
}

impl BuildTarget {
    const fn native(self) -> Option<NativeTarget> {
        match self {
            Self::Web | Self::All => None,
            Self::Macos => Some(NativeTarget::Macos),
            Self::Ios => Some(NativeTarget::Ios),
            Self::Linux => Some(NativeTarget::Linux),
            Self::Windows => Some(NativeTarget::Windows),
            Self::Android => Some(NativeTarget::Android),
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "ty",
    version = tachyon_contracts::PRODUCT_VERSION,
    about = "Tachyon CLI",
    disable_version_flag = true,
    disable_help_subcommand = true,
    help_template = "ty {version} — Tachyon CLI\n\nUsage: ty <command> [options]\n\nCommands:\n  init [name]        Create a new Tachyon app (asks questions in a terminal)\n  add                Add a language, a target or an example to an app\n  preview            Development server: Tac, Yon, or both\n  start              Serve a built bundle with Yon in production\n  bundle             Build client + native artifacts\n  bundle --native    Stage the native host without packaging\n  serve              Compatibility development-server command\n  cache [status|clean] Inspect or clear standalone runtime cache\n  test               Run the project's Test units\n  types              Write TypeScript globals for composed classes\n\nRun 'ty <command> --help' for command-specific options.\n"
)]
struct Cli {
    #[arg(short = 'v', long = "version", global = true)]
    version: bool,
    #[arg(short = 'V', global = true, hide = true)]
    version_upper: bool,
    #[arg(long, global = true, value_enum, default_value_t = DiagnosticFormat::Human)]
    diagnostic_format: DiagnosticFormat,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(
        about = "Create a new Tachyon app. In a terminal it asks questions; every answer is also an option",
        display_order = 1
    )]
    Init {
        #[arg(help = "Missing or empty target directory", default_value = ".")]
        target: PathBuf,
        #[arg(
            help = "Human-readable application name",
            long,
            visible_alias = "app-name"
        )]
        name: Option<String>,
        #[command(flatten)]
        choices: InitChoices,
    },
    #[command(
        about = "Add a language, a target or an example to an existing app",
        display_order = 2
    )]
    Add {
        #[command(subcommand)]
        what: AddWhat,
    },
    #[command(about = "Build client and native artifacts", display_order = 3)]
    Bundle {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(
            help = "Project-relative output directory. Defaults to dist/web for web and dist for native targets",
            long
        )]
        out_dir: Option<PathBuf>,
        #[arg(help = "Disable verified incremental route reuse", long)]
        no_incremental: bool,
        #[arg(
            help = "Rebuild when project sources change",
            long,
            visible_alias = "bundle-watch"
        )]
        watch: bool,
        #[arg(help = "Packaging target; native targets host the client-rendered bundle", long, visible_alias = "targets", value_enum, value_delimiter = ',', num_args = 1..)]
        target: Vec<BuildTarget>,
        #[arg(
            help = "Stage one native host without invoking the platform packager",
            long
        )]
        native: bool,
        #[arg(
            help = "Compatibility switch; native hosts are always generated now",
            long,
            hide = true
        )]
        skip_native_host: bool,
        #[arg(
            help = "Start a watcher without performing its initial build",
            long,
            hide = true
        )]
        skip_initial_build: bool,
        #[arg(
            help = "Compatibility switch; strict-CSP safety is always enforced",
            long,
            hide = true
        )]
        csp_check: bool,
        #[arg(
            help = "Compatibility switch; packaging is already enabled by default",
            long,
            hide = true
        )]
        package: bool,
        #[arg(
            help = "Stage native source and resources without invoking the platform packager",
            long,
            hide = true
        )]
        skip_package: bool,
        #[arg(
            help = "Deprecated render-mode switch; rendering is fixed by the target contract",
            long,
            hide = true
        )]
        render_mode: Option<String>,
    },
    #[command(about = "Rewrite-internal deterministic build command", hide = true)]
    Build {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(
            help = "Project-relative output directory",
            long,
            default_value = "dist"
        )]
        out_dir: PathBuf,
        #[arg(help = "Disable verified incremental route reuse", long)]
        no_incremental: bool,
        #[arg(help = "Packaging target; native targets host the client-rendered bundle", long, value_enum, default_value_t = BuildTarget::Web)]
        target: BuildTarget,
    },
    #[command(about = "Generate a native host application", display_order = 4)]
    NativeBundle {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(help = "Project-relative output directory", long)]
        out_dir: Option<PathBuf>,
        #[arg(help = "Disable verified incremental route reuse", long)]
        no_incremental: bool,
        #[arg(help = "Native packaging target", long, visible_alias = "targets", value_enum, value_delimiter = ',', num_args = 1..)]
        target: Vec<BuildTarget>,
        #[arg(
            help = "Deprecated render-mode switch; rendering is fixed by the target contract",
            long,
            hide = true
        )]
        render_mode: Option<String>,
    },
    #[command(
        about = "Run the Tachyon server",
        name = "serve",
        alias = "dev",
        display_order = 2
    )]
    Dev {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(help = "Interface address", long, visible_alias = "hostname")]
        host: Option<String>,
        #[arg(help = "TCP port; zero requests an ephemeral port", long)]
        port: Option<u16>,
        #[arg(help = "Explicitly permit exposure beyond the local machine", long)]
        allow_non_loopback: bool,
        #[arg(help = "Project-relative output directory", long)]
        out_dir: Option<PathBuf>,
        #[arg(help = "Do not rebuild when sources change", long)]
        no_watch: bool,
        #[arg(
            help = "Keep running the old ty after the ty file on disk changes",
            long
        )]
        no_self_restart: bool,
        #[arg(long, hide = true)]
        check_project: bool,
        #[arg(help = "Serve the existing bundle without compiling it first", long)]
        no_bundle: bool,
        #[arg(
            help = "Compatibility alias; source watching is already enabled by default",
            long,
            hide = true
        )]
        bundle_watch: bool,
    },
    #[command(about = "Serve an existing bundle and supervised Yon handlers without rebuilding")]
    Start {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(help = "Interface address", long, visible_alias = "hostname")]
        host: Option<String>,
        #[arg(help = "TCP port; zero requests an ephemeral port", long)]
        port: Option<u16>,
        #[arg(help = "Explicitly permit exposure beyond the local machine", long)]
        allow_non_loopback: bool,
        #[arg(help = "Project-relative existing output directory", long)]
        out_dir: Option<PathBuf>,
    },
    #[command(
        about = "Run the Tac/Yon development server, or preview an existing static bundle",
        display_order = 5
    )]
    Preview {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(help = "Interface address", long, visible_alias = "hostname")]
        host: Option<String>,
        #[arg(help = "TCP port; zero requests an ephemeral port", long)]
        port: Option<u16>,
        #[arg(help = "Explicitly permit exposure beyond the local machine", long)]
        allow_non_loopback: bool,
        #[arg(
            help = "Project-relative output directory",
            long,
            default_value = "dist"
        )]
        out_dir: PathBuf,
        #[arg(
            help = "Preview existing static files only, without building or running Yon",
            long = "static"
        )]
        static_only: bool,
        #[arg(
            help = "Rebuild the selected target when sources change",
            long,
            visible_alias = "bundle-watch"
        )]
        watch: bool,
        #[arg(
            help = "Keep running the old ty after the ty file on disk changes",
            long
        )]
        no_self_restart: bool,
        #[arg(long, hide = true)]
        check_project: bool,
        #[arg(
            help = "Built target whose embedded web bundle should be served",
            long,
            value_enum
        )]
        target: Option<BuildTarget>,
        #[arg(
            help = "Compatibility switch; the rewrite preview does not require native toolchains",
            long,
            hide = true
        )]
        skip_native_checks: bool,
    },
    #[command(
        about = "Report whether this machine has the toolchains a project needs",
        after_help = "Exit status:\n  0  every toolchain the project needs to build and serve is ready\n  1  a needed toolchain is missing or too old, or the project cannot be read\n\nToolchains needed only to bundle a native target are reported as info and do not change the exit status.",
        hide = true
    )]
    Doctor {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(
            help = "Emit the machine-readable report instead of the human summary",
            long
        )]
        json: bool,
        #[arg(
            help = "Offer to install each missing tool that Tachyon can install, then report",
            long,
            conflicts_with = "json"
        )]
        install: bool,
        #[arg(
            help = "With --install, install every missing tool without asking",
            long,
            requires = "install"
        )]
        yes: bool,
    },
    #[command(about = "Inspect or clear the installation cache", display_order = 6)]
    Cache {
        #[command(subcommand)]
        command: Option<CacheCommand>,
    },
    #[command(about = "Run the project's `Test` units")]
    Test {
        #[arg(help = "Project root holding server/tests", default_value = ".")]
        project: PathBuf,
        #[arg(
            help = "Fail when a public method of an annotated class no case names",
            long
        )]
        coverage: bool,
    },
    #[command(
        about = "Write TypeScript globals for the JavaScript and TypeScript classes Tachyon composes"
    )]
    Types {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
    },
    #[command(about = "Save or check the API that callers of the last release rely on")]
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
    #[command(about = "Invoke and inspect supervised Yon handlers", hide = true)]
    Handler {
        #[command(subcommand)]
        command: HandlerCommand,
    },
    #[command(
        about = "Analyze a legacy project for migration to this implementation",
        hide = true
    )]
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
    #[command(about = "Print command help", hide = true)]
    Help {
        #[arg(help = "Optional command whose help should be printed")]
        command: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum CacheCommand {
    #[command(about = "Report where the cache lives and what it holds")]
    Status,
    #[command(about = "Remove the cache directory")]
    Clean,
}

#[derive(Debug, Subcommand)]
enum ApiCommand {
    #[command(
        about = "Save the current API as the baseline in server/api-baseline.json",
        long_about = "Save the current API as the baseline in server/api-baseline.json.\n\nRun it after a release and commit the file. Every later build compares its API with this baseline and fails when a route, method, field or model is removed, a type is narrowed, a new required field appears or a status changes, unless the major number of tac.application.version is higher than the baseline's (ADR 0074)."
    )]
    Freeze {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum MigrateCommand {
    #[command(about = "Report every construct that this implementation does not yet support")]
    Check {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(
            help = "Emit the machine-readable report instead of the human summary",
            long
        )]
        json: bool,
        #[arg(
            help = "Exit successfully even when unsupported constructs are found",
            long
        )]
        allow_unsupported: bool,
    },
    #[command(
        about = "Print the automatic fixes for Tachyon's breaking changes and the manual steps",
        long_about = "Print the automatic fixes for Tachyon's breaking changes and the manual steps.\n\nThe fixes delete imports from 'tachyon:behaviors' (ADR 0046) and turn an old root middleware class Handler, whose HTTP method functions all hold the same code, into class Middleware with before(request) and after(request) (ADR 0044). They rename old annotations to their plain English names in every language, move files from the renamed role folders repositories/, configurations/ and mappers/ to stores/, setups/ and converters/, and rename the moved role types and test setups everywhere they are named (ADR 0047). They rename a class file after its one class, or the class after its file when other files use the file name, and split a models file with several models into one file per model, next to the old file (ADR 0067). Files that break the one-class rule, and imports, middleware or moves that cannot be changed safely, are listed as manual steps. Nothing is written without --write. Files are only moved inside client/ and server/, never deleted, and symbolic links are never followed."
    )]
    Fix {
        #[arg(help = "Tachyon project root", default_value = ".")]
        project: PathBuf,
        #[arg(help = "Write the printed changes to the project files", long)]
        write: bool,
    },
}

#[derive(Debug, Subcommand)]
enum HandlerCommand {
    #[command(about = "Invoke one Yon handler through Handler Protocol v1")]
    Invoke {
        #[arg(help = "Project-relative controller or worker source file")]
        source: PathBuf,
        #[arg(help = "Tachyon project root", long, default_value = ".")]
        project: PathBuf,
        #[arg(
            help = "Canonical route supplied to the handler",
            long,
            default_value = "/"
        )]
        route: String,
        #[arg(help = "Uppercase HTTP method", long, value_enum, ignore_case = true, default_value_t = CliHttpMethod::Get)]
        method: CliHttpMethod,
        #[arg(
            help = "Stable request correlation identifier",
            long,
            default_value = "cli_request"
        )]
        request_id: String,
        #[arg(help = "Optional UTF-8 request body", long)]
        body: Option<String>,
        #[arg(
            help = "Repeated Handler Protocol header in `name=value` form",
            long = "header"
        )]
        headers: Vec<HeaderArgument>,
        #[arg(help = "Complete invocation deadline in milliseconds", long, default_value_t = 30_000, value_parser = clap::value_parser!(u64).range(1..=300_000))]
        timeout_ms: u64,
        #[arg(
            help = "Explicit environment variable name to inherit",
            long = "allow-env"
        )]
        allow_environment: Vec<String>,
        #[arg(
            help = "Decorator-capable JavaScript executable name or path",
            long,
            default_value = "node"
        )]
        javascript_runtime: PathBuf,
        #[arg(help = "`CPython` executable name or path", long)]
        python_runtime: Option<PathBuf>,
        #[arg(help = "Ruby executable name or path", long)]
        ruby_runtime: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CliHttpMethod {
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
}

impl From<CliHttpMethod> for HttpMethod {
    fn from(method: CliHttpMethod) -> Self {
        match method {
            CliHttpMethod::Delete => Self::Delete,
            CliHttpMethod::Get => Self::Get,
            CliHttpMethod::Head => Self::Head,
            CliHttpMethod::Options => Self::Options,
            CliHttpMethod::Patch => Self::Patch,
            CliHttpMethod::Post => Self::Post,
            CliHttpMethod::Put => Self::Put,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HeaderArgument {
    name: String,
    value: String,
}

impl FromStr for HeaderArgument {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((name, value)) = value.split_once('=') else {
            return Err(String::from("header must use name=value"));
        };
        if name.is_empty() {
            return Err(String::from("header name cannot be empty"));
        }
        Ok(Self {
            name: String::from(name),
            value: String::from(value),
        })
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.version || cli.version_upper {
        println!("{}", tachyon_contracts::PRODUCT_VERSION);
        return;
    }
    let Some(command) = &cli.command else {
        let mut root = Cli::command();
        let _ = root.print_help();
        println!();
        std::process::exit(1);
    };
    let result = execute(command).await;
    if let Err(failure) = result {
        print_failure(&failure, cli.diagnostic_format);
        std::process::exit(1);
    }
}

async fn execute(command: &Command) -> Result<(), Failure> {
    match command {
        Command::Init {
            target,
            name,
            choices,
        } => InitCommand::run(target, name.as_deref(), choices),
        Command::Add { what } => AddCommand::run(what),
        Command::Bundle { .. } => execute_bundle_command(command).await,
        Command::Build {
            project,
            out_dir,
            no_incremental,
            target,
        } => {
            print_simpler_forms(project);
            execute_build(project, out_dir, *no_incremental, *target).await
        }
        Command::NativeBundle { .. } => execute_native_bundle_command(command).await,
        Command::Dev { .. } => execute_dev_command(command).await,
        Command::Start { .. } => execute_start_command(command).await,
        Command::Preview { .. } => execute_preview_command(command).await,
        Command::Doctor {
            project,
            json,
            install,
            yes,
        } => {
            if *install {
                tool_step::ToolStep::install_for_project(project, *yes)?;
            }
            execute_doctor(project, *json)
        }
        Command::Cache { command } => execute_cache(command.as_ref()),
        Command::Test { project, coverage } => execute_test(project, *coverage).await,
        Command::Types { project } => execute_types(project),
        Command::Api {
            command: ApiCommand::Freeze { project },
        } => execute_api_freeze(project),
        Command::Handler { command } => execute_handler(command).await,
        Command::Migrate { command } => execute_migrate(command),
        Command::Help { command } => {
            execute_help(command.as_deref());
            Ok(())
        }
    }
}

fn execute_api_freeze(project: &Path) -> Result<(), Failure> {
    let path = ApiBaselineCheck::freeze(project)?;
    println!("Wrote the API baseline {}", path.display());
    Ok(())
}

fn execute_types(project: &Path) -> Result<(), Failure> {
    ComposedTypes::write(project)?;
    println!(
        "Wrote .tachyon/types/client/composed.d.ts and .tachyon/types/server/composed.d.ts in {}",
        project.display()
    );
    Ok(())
}

fn execute_help(command: Option<&str>) {
    let mut root = Cli::command();
    if let Some(name) = command
        && let Some(subcommand) = root.find_subcommand_mut(name)
    {
        let _ = subcommand.print_long_help();
        println!();
        return;
    }
    let _ = root.print_long_help();
    println!();
}

async fn execute_bundle_command(command: &Command) -> Result<(), Failure> {
    let Command::Bundle {
        project,
        out_dir,
        no_incremental,
        watch,
        target,
        native,
        skip_initial_build,
        render_mode,
        skip_package,
        ..
    } = command
    else {
        unreachable!();
    };
    reject_render_mode(render_mode.as_deref())?;
    let targets = resolve_targets(
        target,
        &["TAC_BUNDLE_TARGET", "TAC_TARGET"],
        if *native {
            None
        } else {
            Some(BuildTarget::Web)
        },
    )?;
    if *native {
        require_one_native_target(&targets, "bundle --native")?;
    }
    let output = bundle_output(out_dir.as_deref(), &targets);
    let options = BundleExecution {
        no_incremental: *no_incremental,
        watch: *watch,
        initial_build: if *skip_initial_build {
            InitialBuild::Skip
        } else {
            InitialBuild::Run
        },
        package: !*skip_package && !*native,
    };
    execute_bundle(project, &output, &targets, options).await
}

async fn execute_native_bundle_command(command: &Command) -> Result<(), Failure> {
    let Command::NativeBundle {
        project,
        out_dir,
        no_incremental,
        target,
        render_mode,
    } = command
    else {
        unreachable!();
    };
    reject_render_mode(render_mode.as_deref())?;
    let targets = resolve_targets(target, &["TAC_BUNDLE_TARGET", "TAC_TARGET"], None)?;
    let target = require_one_native_target(&targets, "native-bundle")?;
    let output = out_dir
        .clone()
        .or_else(|| environment_path("TAC_DIST_PATH"))
        .unwrap_or_else(|| PathBuf::from("dist"));
    execute_build_with_package(project, &output, *no_incremental, target, false).await
}

async fn execute_dev_command(command: &Command) -> Result<(), Failure> {
    let Command::Dev {
        project,
        host,
        port,
        allow_non_loopback,
        out_dir,
        no_watch,
        no_bundle,
        no_self_restart,
        check_project,
        ..
    } = command
    else {
        unreachable!();
    };
    if *check_project {
        return application_for_server(project, host.as_deref(), *port, *allow_non_loopback)
            .map(|_| ());
    }
    let output = out_dir
        .clone()
        .or_else(|| environment_path("TAC_DIST_PATH"))
        .unwrap_or_else(|| PathBuf::from("dist/web"));
    let production = std::env::var("NODE_ENV").is_ok_and(|value| value == "production");
    let policy = ServingPolicy {
        watch: !(*no_watch || production),
        build: !(*no_bundle || truthy_environment("YON_SKIP_BUNDLE") || production),
        self_restart: !no_self_restart,
        command: LongRunningCommand::Serve,
    };
    if policy.watch {
        return ResilientServe {
            project: project.clone(),
            host: host.clone(),
            port: *port,
            allow_non_loopback: *allow_non_loopback,
            output,
            policy,
            default_port: 8080,
            advice: AdviceReport::none(),
        }
        .run()
        .await;
    }
    let (project, application) =
        application_for_server(project, host.as_deref(), *port, *allow_non_loopback)?;
    let host = application.host().unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let port = application.port().unwrap_or(8080);
    execute_serve(
        &project,
        host,
        port,
        *allow_non_loopback,
        &output,
        policy,
        application,
    )
    .await
}

async fn execute_start_command(command: &Command) -> Result<(), Failure> {
    let Command::Start {
        project,
        host,
        port,
        allow_non_loopback,
        out_dir,
    } = command
    else {
        unreachable!();
    };
    let (project, application) =
        application_for_server(project, host.as_deref(), *port, *allow_non_loopback)?;
    let host = application.host().unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let port = application.port().unwrap_or(8080);
    let output = out_dir
        .clone()
        .or_else(|| environment_path("TAC_DIST_PATH"))
        .unwrap_or_else(|| PathBuf::from("dist/web"));
    execute_serve(
        &project,
        host,
        port,
        *allow_non_loopback,
        &output,
        ServingPolicy {
            watch: false,
            build: false,
            self_restart: false,
            command: LongRunningCommand::Start,
        },
        application,
    )
    .await
}

async fn execute_preview_command(command: &Command) -> Result<(), Failure> {
    let Command::Preview {
        project,
        host,
        port,
        allow_non_loopback,
        out_dir,
        static_only,
        watch,
        no_self_restart,
        check_project,
        target,
        ..
    } = command
    else {
        unreachable!();
    };
    if *check_project {
        return application_for_server(project, host.as_deref(), *port, *allow_non_loopback)
            .map(|_| ());
    }
    let explicit_targets: Vec<_> = target.iter().copied().collect();
    let targets = resolve_targets(
        &explicit_targets,
        &["TAC_PREVIEW_TARGET", "TAC_TARGET"],
        Some(BuildTarget::Web),
    )?;
    let target = require_one_target(&targets, "preview")?;
    let advice = AdviceReport::simpler_forms(project);
    if target == BuildTarget::Web && !static_only {
        let output = if out_dir == Path::new("dist") {
            environment_path("TAC_DIST_PATH").unwrap_or_else(|| PathBuf::from("dist/web"))
        } else {
            out_dir.clone()
        };
        return ResilientServe {
            project: project.clone(),
            host: host.clone(),
            port: *port,
            allow_non_loopback: *allow_non_loopback,
            output,
            policy: ServingPolicy {
                watch: true,
                build: true,
                self_restart: !no_self_restart,
                command: LongRunningCommand::Preview,
            },
            default_port: 3000,
            advice,
        }
        .run()
        .await;
    }
    advice.start_once();
    execute_preview(
        project,
        resolve_host(host.as_deref(), &[])?,
        resolve_port(*port, &[], 3000)?,
        *allow_non_loopback,
        out_dir,
        *watch,
        target,
    )
    .await
}

#[derive(Clone, Copy)]
struct BundleExecution {
    no_incremental: bool,
    watch: bool,
    initial_build: InitialBuild,
    package: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InitialBuild {
    Run,
    Skip,
}

async fn require_tested_methods(
    project: &Path,
    command: LongRunningCommand,
) -> Result<(), Failure> {
    if command != LongRunningCommand::Preview
        || std::env::var("TAC_ALLOW_UNTESTED").is_ok_and(|value| value == "1")
    {
        return Ok(());
    }
    let calls = match RecordedCalls::read_current(project) {
        Some(calls) => calls,
        None => record_test_calls(project).await?,
    };
    CoverageGate::refuse_untested(project, calls)
}

async fn record_test_calls(project: &Path) -> Result<RecordedCalls, Failure> {
    ComposedTypes::write(project)?;
    RecordedCalls::invalidate(project)?;
    let mut calls = match run_test_units(project, TestOutput::Quiet).await? {
        Some(results) => results.preview_calls()?,
        None => RecordedCalls::default(),
    };
    calls.set_successful(true);
    calls.write(project)?;
    Ok(calls)
}

#[derive(Clone, Copy)]
struct ServingPolicy {
    watch: bool,
    build: bool,
    self_restart: bool,
    command: LongRunningCommand,
}

fn application_for_server(
    project: &Path,
    host: Option<&str>,
    port: Option<u16>,
    allow_non_loopback: bool,
) -> Result<(tachyon_core::Project, ApplicationConfig), Failure> {
    let host = host
        .map(|value| resolve_host(Some(value), &[]))
        .transpose()?;
    if !allow_non_loopback && host.is_some_and(|host| !host.is_loopback()) {
        return Err(cli_failure(
            1301,
            "Refusing to expose the server without explicit permission.",
            "Use a loopback host or pass --allow-non-loopback.",
        ));
    }
    let project = tachyon_core::ProjectDiscovery::discover(project)?;
    let application = ApplicationConfig::load_project_for_server(&project, host, port)?;
    Ok((project, application))
}

async fn execute_serve(
    project: &tachyon_core::Project,
    host: IpAddr,
    port: u16,
    allow_non_loopback: bool,
    out_dir: &Path,
    policy: ServingPolicy,
    application: ApplicationConfig,
) -> Result<(), Failure> {
    tachyon_core::cache::ensure_runtime()?;
    let options = dev_server_options(host, port, allow_non_loopback, out_dir, policy);
    let server = DevServer::bind_project_with_application(project, &options, application).await?;
    let shutdown = ShutdownSignals::install(policy.command);
    serve_dev_server(server, policy, shutdown.wait()).await
}

fn dev_server_options(
    host: IpAddr,
    port: u16,
    allow_non_loopback: bool,
    out_dir: &Path,
    policy: ServingPolicy,
) -> DevServerOptions {
    DevServerOptions {
        host,
        port,
        allow_non_loopback,
        output_directory: out_dir.to_path_buf(),
        watch: policy.watch,
        build: policy.build,
        restart_executable: policy
            .self_restart
            .then(std::env::current_exe)
            .and_then(Result::ok),
        restart_check_arguments: std::env::args_os()
            .skip(1)
            .chain([std::ffi::OsString::from("--check-project")])
            .collect(),
    }
}

const fn ready_label(command: LongRunningCommand) -> &'static str {
    match command {
        LongRunningCommand::Preview => "Tachyon preview ready",
        LongRunningCommand::Start => "Tachyon production server ready",
        _ => "Tachyon server ready",
    }
}

async fn serve_dev_server(
    server: DevServer,
    policy: ServingPolicy,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), Failure> {
    println!(
        "{} at http://{}/",
        ready_label(policy.command),
        server.address()
    );
    let mut stdout = std::io::stdout();
    if let Some(build) = server.build() {
        let _ = writeln!(stdout, "{}", build.timings_line());
    } else if policy.build {
        let _ = writeln!(
            stdout,
            "Serving the last build while the sources rebuild in the background."
        );
    }
    if policy.watch {
        let _ = writeln!(
            stdout,
            "Watching sources; open pages receive semantic hot updates."
        );
    }
    let _flush_result = std::io::stdout().flush();
    let tool_restart = server.tool_restart();
    let restart_requested = tool_restart.clone();
    let run_result = server
        .run_until(async move {
            let Some(restart_requested) = restart_requested else {
                shutdown.await;
                return;
            };
            tokio::select! {
                () = shutdown => {}
                () = restart_requested.requested() => {}
            }
        })
        .await;
    match tool_restart {
        Some(tool_restart) if tool_restart.is_requested() => {
            run_result?;
            Err(restart::SelfRestart::replace_process(&tool_restart))
        }
        _ => run_result,
    }
}

async fn execute_preview(
    project: &Path,
    host: IpAddr,
    port: u16,
    allow_non_loopback: bool,
    out_dir: &Path,
    watch: bool,
    target: BuildTarget,
) -> Result<(), Failure> {
    let build_output = if watch {
        let build_output = preview_build_output(out_dir, target);
        execute_build(project, &build_output, false, target).await?;
        Some(build_output)
    } else {
        None
    };

    let root = resolve_preview_root(project, out_dir, target);
    let server = PreviewServer::bind(
        &root,
        &PreviewServerOptions {
            host,
            port,
            allow_non_loopback,
        },
    )
    .await?;
    let shutdown = ShutdownSignals::install(LongRunningCommand::Preview);
    let watcher = build_output.map(|build_output| {
        PreviewBundleWatcher::start(project.to_path_buf(), build_output, target)
    });
    let watcher_cancellation = watcher
        .as_ref()
        .map(PreviewBundleWatcher::cancellation_handle);
    println!(
        "Tachyon preview ready at http://{}/ ({})",
        server.address(),
        server.root().display()
    );
    if watch {
        let _ = writeln!(
            std::io::stdout(),
            "Watching sources and rebuilding the preview."
        );
    }
    let _flush_result = std::io::stdout().flush();
    let server_result = server
        .run_until(async move {
            shutdown.wait().await;
            if let Some(cancellation) = watcher_cancellation {
                cancellation.send_replace(true);
            }
        })
        .await;
    let watcher_result = if let Some(watcher) = watcher {
        watcher.cancel_and_join().await
    } else {
        Ok(())
    };
    server_result?;
    watcher_result
}

async fn execute_bundle(
    project: &Path,
    out_dir: &Path,
    targets: &[BuildTarget],
    options: BundleExecution,
) -> Result<(), Failure> {
    tachyon_core::cache::ensure_runtime()?;
    if !options.watch || options.initial_build == InitialBuild::Run {
        execute_build_targets(
            project,
            out_dir,
            options.no_incremental,
            targets,
            options.package,
        )
        .await?;
    }
    if !options.watch {
        return Ok(());
    }

    let shutdown = ShutdownSignals::install(LongRunningCommand::BundleWatch);
    println!("Watching sources and rebuilding the bundle.");
    let mut fingerprint = source_fingerprint(project);
    let shutdown = shutdown.wait();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            () = tokio::time::sleep(Duration::from_millis(400)) => {
                let next = source_fingerprint(project);
                if next == fingerprint {
                    continue;
                }
                fingerprint = next;
                if let Err(failure) = execute_build_targets(project, out_dir, options.no_incremental, targets, options.package).await {
                    eprint!("{failure}");
                }
            }
        }
    }
}

async fn execute_build_targets(
    project: &Path,
    out_dir: &Path,
    no_incremental: bool,
    targets: &[BuildTarget],
    package: bool,
) -> Result<(), Failure> {
    let multiple = targets.len() > 1;
    for target in targets {
        let output = if multiple && *target == BuildTarget::Web {
            out_dir.join(target_directory(BuildTarget::Web))
        } else {
            out_dir.to_path_buf()
        };
        execute_build_with_package(project, &output, no_incremental, *target, package).await?;
    }
    Ok(())
}

const fn target_directory(target: BuildTarget) -> &'static str {
    match target {
        BuildTarget::Web => "web",
        BuildTarget::Macos => "macos",
        BuildTarget::Ios => "ios",
        BuildTarget::Linux => "linux",
        BuildTarget::Windows => "windows",
        BuildTarget::Android => "android",
        BuildTarget::All => "all",
    }
}

fn bundle_output(explicit: Option<&Path>, targets: &[BuildTarget]) -> PathBuf {
    if let Some(output) = explicit {
        return output.to_path_buf();
    }
    if let Some(output) = environment_path("TAC_DIST_PATH") {
        return output;
    }
    if targets == [BuildTarget::Web] {
        PathBuf::from("dist/web")
    } else {
        PathBuf::from("dist")
    }
}

fn resolve_targets(
    explicit: &[BuildTarget],
    environment_names: &[&str],
    default: Option<BuildTarget>,
) -> Result<Vec<BuildTarget>, Failure> {
    let requested = if explicit.is_empty() {
        let environment = environment_names.iter().find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
        if let Some(environment) = environment {
            parse_target_list(&environment)?
        } else {
            default.into_iter().collect()
        }
    } else {
        explicit.to_vec()
    };

    let mut resolved = Vec::new();
    for target in requested {
        let expansion: &[BuildTarget] = if target == BuildTarget::All {
            &[
                BuildTarget::Web,
                BuildTarget::Macos,
                BuildTarget::Windows,
                BuildTarget::Linux,
                BuildTarget::Android,
                BuildTarget::Ios,
            ]
        } else {
            std::slice::from_ref(&target)
        };
        for expanded in expansion {
            if !resolved.contains(expanded) {
                resolved.push(*expanded);
            }
        }
    }
    Ok(resolved)
}

fn parse_target_list(value: &str) -> Result<Vec<BuildTarget>, Failure> {
    value
        .split(|character: char| character == ',' || character.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .map(parse_target)
        .collect()
}

fn parse_target(value: &str) -> Result<BuildTarget, Failure> {
    match value.to_ascii_lowercase().as_str() {
        "web" | "browser" => Ok(BuildTarget::Web),
        "macos" | "mac" | "darwin" | "osx" => Ok(BuildTarget::Macos),
        "ios" => Ok(BuildTarget::Ios),
        "linux" => Ok(BuildTarget::Linux),
        "windows" | "win" | "win32" => Ok(BuildTarget::Windows),
        "android" => Ok(BuildTarget::Android),
        "all" => Ok(BuildTarget::All),
        _ => Err(cli_failure(
            1001,
            format!("Unknown bundle target '{value}'."),
            "Use web, macos, windows, linux, android, ios, or all.",
        )),
    }
}

fn require_one_target(targets: &[BuildTarget], command: &str) -> Result<BuildTarget, Failure> {
    if let [target] = targets {
        return Ok(*target);
    }
    Err(cli_failure(
        1001,
        format!("The {command} command requires exactly one target."),
        format!("Pass one target with `ty {command} --target <target>`."),
    ))
}

fn require_one_native_target(
    targets: &[BuildTarget],
    command: &str,
) -> Result<BuildTarget, Failure> {
    let target = require_one_target(targets, command)?;
    if target.native().is_some() {
        return Ok(target);
    }
    Err(cli_failure(
        1001,
        format!("The {command} command requires a native target."),
        "Use macos, windows, linux, android, or ios.",
    ))
}

fn environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn truthy_environment(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| matches!(value.as_str(), "1" | "true"))
}

fn reject_render_mode(explicit: Option<&str>) -> Result<(), Failure> {
    if explicit.is_some()
        || std::env::var("TAC_RENDER_MODE").is_ok_and(|value| !value.trim().is_empty())
    {
        return Err(cli_failure(
            1001,
            "--render-mode and TAC_RENDER_MODE have been removed.",
            "Native targets always host the client-rendered bundle in the platform web view; use target-native page companions for platform behavior (ADR 0019).",
        ));
    }
    Ok(())
}

fn resolve_host(explicit: Option<&str>, environment_names: &[&str]) -> Result<IpAddr, Failure> {
    let value = explicit.map(String::from).or_else(|| {
        environment_names.iter().find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
    });
    let Some(value) = value else {
        return Ok(IpAddr::from([127, 0, 0, 1]));
    };
    if value.eq_ignore_ascii_case("localhost") {
        return Ok(IpAddr::from([127, 0, 0, 1]));
    }
    value.parse().map_err(|_| {
        cli_failure(
            1302,
            format!("Invalid host address '{value}'."),
            "Use an IPv4 or IPv6 address, or localhost.",
        )
    })
}

fn resolve_port(
    explicit: Option<u16>,
    environment_names: &[&str],
    default: u16,
) -> Result<u16, Failure> {
    if let Some(port) = explicit {
        return Ok(port);
    }
    let Some((name, value)) = environment_names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| (*name, value))
    }) else {
        return Ok(default);
    };
    value.parse().map_err(|_| {
        cli_failure(
            1302,
            format!("Invalid port '{value}' from {name}."),
            "Use a TCP port from 0 through 65535.",
        )
    })
}

fn cli_failure(number: u16, message: impl Into<String>, help: impl Into<String>) -> Failure {
    Failure::one(Diagnostic {
        code: DiagnosticCode::from_number(number).unwrap_or_else(|| unreachable!()),
        severity: Severity::Error,
        message: message.into(),
        help: Some(help.into()),
        spans: Vec::new(),
    })
}

struct PreviewBundleWatcher {
    cancellation: tokio::sync::watch::Sender<bool>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl PreviewBundleWatcher {
    fn start(project: PathBuf, out_dir: PathBuf, target: BuildTarget) -> Self {
        let (cancellation, mut cancelled) = tokio::sync::watch::channel(false);
        let mut fingerprint = source_fingerprint(&project);
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = preview_watcher_cancelled(&mut cancelled) => return,
                    () = tokio::time::sleep(Duration::from_millis(400)) => {}
                }
                let next = source_fingerprint(&project);
                if next == fingerprint {
                    continue;
                }
                fingerprint = next;
                let build = execute_build(&project, &out_dir, false, target);
                tokio::pin!(build);
                tokio::select! {
                    biased;
                    () = preview_watcher_cancelled(&mut cancelled) => return,
                    result = &mut build => {
                        if let Err(failure) = result {
                            eprint!("{failure}");
                        }
                    }
                }
            }
        });
        Self {
            cancellation,
            task: Some(task),
        }
    }

    fn cancellation_handle(&self) -> tokio::sync::watch::Sender<bool> {
        self.cancellation.clone()
    }

    async fn cancel_and_join(mut self) -> Result<(), Failure> {
        self.cancellation.send_replace(true);
        let Some(task) = self.task.take() else {
            return Ok(());
        };
        task.await.map_err(|error| {
            cli_failure(
                1303,
                format!("Preview rebuild watcher stopped unexpectedly: {error}"),
                "Restart the preview server.",
            )
        })
    }
}

impl Drop for PreviewBundleWatcher {
    fn drop(&mut self) {
        self.cancellation.send_replace(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn preview_watcher_cancelled(cancelled: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *cancelled.borrow() {
            return;
        }
        if cancelled.changed().await.is_err() {
            return;
        }
    }
}

fn source_fingerprint(project: &Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut pending = vec![project.join("client"), project.join("server")];
    pending.push(project.join("package.json"));
    while let Some(current) = pending.pop() {
        let Ok(metadata) = std::fs::symlink_metadata(&current) else {
            continue;
        };
        if metadata.is_dir() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            let mut children: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
            children.sort();
            pending.extend(children);
            continue;
        }
        current.hash(&mut hasher);
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            elapsed.as_nanos().hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn preview_build_output(out_dir: &Path, target: BuildTarget) -> PathBuf {
    if target == BuildTarget::Web && out_dir == Path::new("dist") {
        out_dir.join("web")
    } else {
        out_dir.to_path_buf()
    }
}

fn resolve_preview_root(project: &Path, out_dir: &Path, target: BuildTarget) -> PathBuf {
    let output = project.join(out_dir);
    let direct = output.join("index.html");
    if target != BuildTarget::Web && direct.is_file() {
        return output;
    }

    let target_name = match target {
        BuildTarget::Web | BuildTarget::All => "web",
        BuildTarget::Macos => "macos",
        BuildTarget::Ios => "ios",
        BuildTarget::Linux => "linux",
        BuildTarget::Windows => "windows",
        BuildTarget::Android => "android",
    };
    let target_root = output.join(target_name);
    for candidate in [target_root.join("web"), target_root.clone()] {
        if candidate.join("index.html").is_file() {
            return candidate;
        }
    }

    if let Ok(entries) = std::fs::read_dir(&target_root) {
        for app in entries.flatten().map(|entry| entry.path()) {
            for candidate in [
                app.join("WebBundle"),
                app.join("Contents/Resources/WebBundle"),
                app.join("Resources/WebBundle"),
            ] {
                if candidate.join("index.html").is_file() {
                    return candidate;
                }
            }
        }
    }

    if target == BuildTarget::Web && direct.is_file() {
        return output;
    }
    target_root
}

async fn execute_build(
    project: &Path,
    out_dir: &Path,
    no_incremental: bool,
    target: BuildTarget,
) -> Result<(), Failure> {
    execute_build_with_package(project, out_dir, no_incremental, target, true).await
}

async fn execute_build_with_package(
    project: &Path,
    out_dir: &Path,
    no_incremental: bool,
    target: BuildTarget,
    package: bool,
) -> Result<(), Failure> {
    if target == BuildTarget::All {
        execute_web_build(project, &out_dir.join("web"), no_incremental).await?;
        for native in [
            NativeTarget::Macos,
            NativeTarget::Windows,
            NativeTarget::Linux,
            NativeTarget::Android,
            NativeTarget::Ios,
        ] {
            execute_native_build(project, out_dir, native, package).await?;
        }
        return Ok(());
    }
    if let Some(native) = target.native() {
        return execute_native_build(project, out_dir, native, package).await;
    }

    execute_web_build(project, out_dir, no_incremental).await
}

async fn execute_web_build(
    project: &Path,
    out_dir: &Path,
    no_incremental: bool,
) -> Result<(), Failure> {
    let result = WebCompiler::build_async(
        project,
        &BuildOptions {
            output_directory: out_dir.to_path_buf(),
            incremental: !no_incremental,
            ..BuildOptions::default()
        },
    )
    .await?;
    let noun = if result.route_count() == 1 {
        "route"
    } else {
        "routes"
    };
    println!(
        "Built {} {noun} to {} ({}) compiled={} reused={}",
        result.route_count(),
        result.output_directory().display(),
        result.sha256(),
        result.compiled_routes(),
        result.reused_routes()
    );
    if let Some(line) = result.compression().line() {
        println!("{line}");
    }
    for note in result.service_notes() {
        println!("{note}");
    }
    Ok(())
}

async fn execute_native_build(
    project: &Path,
    out_dir: &Path,
    native: NativeTarget,
    package: bool,
) -> Result<(), Failure> {
    let result = NativeCompiler::build_async(
        project,
        &NativeBuildOptions {
            output_directory: out_dir.to_path_buf(),
            target: native,
            package,
        },
    )
    .await?;
    println!(
        "Built {} app hosting {} routes to {} ({})",
        native_target_directory(native),
        result.route_count(),
        result.application_bundle().display(),
        result.sha256()
    );
    Ok(())
}

fn execute_doctor(project: &PathBuf, json: bool) -> Result<(), Failure> {
    let report = tachyon_core::doctor::check(project)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| String::from("{}"))
        );
    } else {
        print!("{}", report.to_text());
    }
    if !report.is_ready() {
        let _ = std::io::stdout().flush();
        std::process::exit(1);
    }
    Ok(())
}

fn execute_cache(command: Option<&CacheCommand>) -> Result<(), Failure> {
    if matches!(command, Some(CacheCommand::Clean)) {
        let removed = tachyon_core::cache::clean()?;
        println!(
            "Cleared Tachyon runtime cache: {}",
            removed.root.join("runtime").display()
        );
        return Ok(());
    }

    let status = tachyon_core::cache::status();
    println!(
        "Tachyon cache\n  Root: {}\n  Runtime entries: {}",
        status.root.display(),
        status.runtime_entries
    );
    Ok(())
}

fn execute_migrate(command: &MigrateCommand) -> Result<(), Failure> {
    match command {
        MigrateCommand::Check {
            project,
            json,
            allow_unsupported,
        } => execute_migrate_check(project, *json, *allow_unsupported),
        MigrateCommand::Fix { project, write } => execute_migrate_fix(project, *write),
    }
}

fn execute_migrate_fix(project: &Path, write: bool) -> Result<(), Failure> {
    let plan = MigrationFix::plan(project)?;
    if write {
        plan.write(project)?;
    }
    print!("{}", plan.to_text(write));
    Ok(())
}

fn execute_migrate_check(
    project: &Path,
    json: bool,
    allow_unsupported: bool,
) -> Result<(), Failure> {
    let report = MigrationAnalysis::check(project)?;
    if json {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        if serde_json::to_writer_pretty(&mut lock, &report).is_ok() {
            let _newline_result = writeln!(lock);
        }
    } else {
        print!("{}", report.to_text());
    }
    if allow_unsupported {
        return Ok(());
    }
    report.unsupported_failure().map_or(Ok(()), Err)
}

async fn execute_test(project: &Path, coverage: bool) -> Result<(), Failure> {
    let resolved = project.canonicalize().map_err(|error| {
        cli_failure(
            2021,
            format!("Cannot resolve the project directory: {error}"),
            "Pass a directory that exists, or run the command inside the project.",
        )
    })?;
    let project = resolved.as_path();
    print_simpler_forms(project);
    ComposedTypes::write(project)?;
    RecordedCalls::invalidate(project)?;
    let Some(results) = run_test_units(project, TestOutput::Terminal).await? else {
        println!("No test units found. Place <Name>Test.<ext> under server/tests or client/tests.");
        return Ok(());
    };
    results.finish(project, coverage)
}

async fn run_test_units(
    project: &Path,
    output: TestOutput,
) -> Result<Option<TestResults>, Failure> {
    let units = test_units(project);
    let client = tachyon_core::client_tests::units(project, "client");
    if units.is_empty() && client.is_empty() {
        return Ok(None);
    }
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions::from_environment()?)?;
    let extensions = units
        .iter()
        .filter_map(|relative| relative.extension().and_then(|value| value.to_str()))
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    supervisor
        .preflight_languages(extensions.iter().map(String::as_str))
        .await?;
    let mut sources = Vec::new();
    for relative in &units {
        sources.push(HandlerSource::discover(project, relative)?);
    }
    supervisor.preflight_gateways(&sources).await?;
    let mut results = TestResults::new(output);
    for (relative, source) in units.iter().zip(&sources) {
        results
            .run_server_unit(relative, source, &supervisor)
            .await?;
    }
    for relative in client {
        results.run_client_unit(project, &relative);
    }
    Ok(Some(results))
}

struct TestResults {
    passed: usize,
    failed: usize,
    calls: RecordedCalls,
    output: TestOutput,
}

impl TestResults {
    fn new(output: TestOutput) -> Self {
        Self {
            passed: 0,
            failed: 0,
            calls: RecordedCalls::default(),
            output,
        }
    }

    async fn run_server_unit(
        &mut self,
        relative: &Path,
        source: &HandlerSource,
        supervisor: &HandlerSupervisor,
    ) -> Result<(), Failure> {
        let portable = relative.to_string_lossy().replace('\\', "/");
        let response = supervisor
            .invoke(
                source,
                &HandlerRequest::route("ty-test", "/", HttpMethod::Post),
                &HandlerCancellation::default(),
            )
            .await?;
        let language = relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        self.calls
            .add_unit("server", language, response.error.is_none());
        if let Some(error) = response.error {
            self.failed += 1;
            self.output
                .failure(&format!("FAIL {portable}: {}", error.message));
            return Ok(());
        }
        let report = response
            .body
            .and_then(|body| serde_json::from_str::<serde_json::Value>(&body.data).ok())
            .unwrap_or_default();
        let unit = report["unit"].as_str().unwrap_or(&portable);
        for case in report["cases"].as_array().into_iter().flatten() {
            let name = case["name"].as_str().unwrap_or("<case>");
            self.record(
                unit,
                name,
                case["passed"] == serde_json::Value::Bool(true),
                case["error"].as_str(),
            );
            let called: Vec<String> = case["called"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|key| key.as_str().map(str::to_owned))
                .collect();
            self.calls.add("server", &format!("{unit}.{name}"), &called);
        }
        Ok(())
    }

    fn run_client_unit(&mut self, project: &Path, relative: &Path) {
        let portable = relative.to_string_lossy().replace('\\', "/");
        let language = relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let outcome = tachyon_core::client_tests::run(project, relative);
        self.calls.add_unit("client", language, outcome.is_ok());
        let report = match outcome {
            Ok(report) => report,
            Err(failure) => {
                self.failed += 1;
                self.output.failure(&format!(
                    "FAIL {portable}: {}",
                    failure.diagnostics()[0].message
                ));
                return;
            }
        };
        for case in report.cases {
            self.record(&report.unit, &case.name, case.passed, case.error.as_deref());
            let first_case = format!("{}.{}", report.unit, case.name);
            self.calls.add("client", &first_case, &case.called);
        }
    }

    fn record(&mut self, unit: &str, name: &str, passed: bool, error: Option<&str>) {
        if passed {
            self.passed += 1;
            self.output.line(&format!("  ok  {unit}.{name}"));
            return;
        }
        self.failed += 1;
        let reason = error.unwrap_or("failed");
        self.output
            .failure(&format!("FAIL {unit}.{name}: {reason}"));
    }

    fn preview_calls(self) -> Result<RecordedCalls, Failure> {
        if self.failed == 0 {
            return Ok(self.calls);
        }
        Err(Self::failed_cases())
    }

    fn finish(&self, project: &Path, coverage: bool) -> Result<(), Failure> {
        println!("{} passed, {} failed.", self.passed, self.failed);
        let mut calls = self.calls.clone();
        calls.set_successful(self.failed == 0);
        calls.write(project)?;
        let uncovered = CoverageGate::print_test_report(project, self.calls.clone(), coverage)?;
        if self.failed == 0 && !uncovered {
            return Ok(());
        }
        if self.failed == 0 {
            return Err(cli_failure(
                2022,
                "One or more public methods of an annotated class were not called by a test case.",
                "Call each listed method from a test* case, or drop the class-level annotation.",
            ));
        }
        Err(Self::failed_cases())
    }

    fn failed_cases() -> Failure {
        cli_failure(
            2020,
            "One or more test cases failed.",
            "Fix the reported cases, or run one unit with ty handler invoke for its full output.",
        )
    }
}

fn test_units(project: &Path) -> Vec<PathBuf> {
    let mut units = Vec::new();
    let mut pending = vec![PathBuf::from("server/tests")];
    while let Some(relative) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(project.join(&relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let child = relative.join(name);
            if entry.path().is_dir() {
                pending.push(child);
                continue;
            }
            let stem = Path::new(name).file_stem().and_then(|value| value.to_str());
            if stem.is_some_and(|stem| stem.ends_with("Test")) {
                units.push(child);
            }
        }
    }
    units.sort();
    units
}

async fn execute_handler(command: &HandlerCommand) -> Result<(), Failure> {
    let HandlerCommand::Invoke {
        source,
        project,
        method,
        ..
    } = command;
    let (source, application) = HandlerSource::discover_with_application(project, source)?;
    if source.streams_method(HttpMethod::from(*method)) {
        return Err(cli_failure(
            2006,
            "handler invoke cannot represent a multi-frame generator response.",
            "Use ty preview or ty start and request this method over HTTP/SSE.",
        ));
    }
    let supervisor = command.supervisor(application)?;
    let request = command.request();
    let cancellation = handler_cancellation();
    let response = supervisor.invoke(&source, &request, &cancellation).await?;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if serde_json::to_writer_pretty(&mut lock, &response).is_err() {
        return Ok(());
    }
    let _newline_result = writeln!(lock);
    Ok(())
}

impl HandlerCommand {
    fn supervisor(&self, application: ApplicationConfig) -> Result<HandlerSupervisor, Failure> {
        let Self::Invoke {
            allow_environment,
            javascript_runtime,
            python_runtime,
            ruby_runtime,
            timeout_ms,
            ..
        } = self;
        let environment = EnvironmentPolicy::from_names(allow_environment.clone())?;
        let mut runtimes = HandlerRuntimePrograms {
            javascript: javascript_runtime.clone(),
            ..HandlerRuntimePrograms::default()
        };
        if let Some(python_runtime) = python_runtime {
            runtimes.python.clone_from(python_runtime);
        }
        if let Some(ruby_runtime) = ruby_runtime {
            runtimes.ruby.clone_from(ruby_runtime);
        }
        let mut options = HandlerSupervisorOptions::from_environment()?;
        options.default_timeout = Duration::from_millis(*timeout_ms);
        options.runtimes = runtimes;
        options.environment = environment;
        options.application = application;
        HandlerSupervisor::new(options)
    }

    fn request(&self) -> HandlerRequest {
        let Self::Invoke {
            request_id,
            route,
            method,
            timeout_ms,
            headers,
            body,
            ..
        } = self;
        let mut request =
            HandlerRequest::route(request_id.clone(), route.clone(), HttpMethod::from(*method));
        request.deadline_ms = Some(*timeout_ms);
        for header in headers {
            request
                .headers
                .entry(header.name.clone())
                .or_default()
                .push(header.value.clone());
        }
        request.body = body.as_ref().map(|data| HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: data.clone(),
        });
        request
    }
}

fn handler_cancellation() -> HandlerCancellation {
    let cancellation = HandlerCancellation::default();
    let signal = cancellation.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal.cancel();
        }
    });
    cancellation
}

fn print_simpler_forms(project: &Path) {
    eprint!("{}", tachyon_core::SimplerForms::report(project));
}

fn print_failure(failure: &Failure, format: DiagnosticFormat) {
    match format {
        DiagnosticFormat::Human => eprint!("{failure}"),
        DiagnosticFormat::Json => {
            let stderr = std::io::stderr();
            let mut lock = stderr.lock();
            if serde_json::to_writer_pretty(&mut lock, &failure.report()).is_err() {
                eprint!("{failure}");
                return;
            }
            let _newline_result = writeln!(lock);
        }
    }
}
