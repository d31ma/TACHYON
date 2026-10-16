mod needed_services;
mod ruby_model_readers;
mod runtime_build;
mod runtime_sources;
mod runtime_tools;

pub(crate) use needed_services::NeededServices;
pub(crate) use runtime_build::RubyRuntimeBuild;

use crate::Failure;
use crate::events::EventModule;
use crate::failure::diagnostic;
use crate::ruby_source::RubyMembers;
use ruby_model_readers::RubyModelReaders;
use runtime_sources::RubyRuntimeSources;
use std::path::{Path, PathBuf};

pub(crate) struct RubyWebServices;

impl RubyWebServices {
    pub(crate) const PRELUDE: &'static str = concat!(
        include_str!("tac_ruby/prelude.rb"),
        include_str!("tac_ruby/web_host.rb"),
        include_str!("tac_ruby/tasks.rb"),
        include_str!("tac_ruby/browser_fakes.rb"),
        include_str!("tac_ruby/clock.rb"),
        include_str!("tac_ruby/secure_random.rb"),
        include_str!("tac_ruby/connectivity.rb"),
        include_str!("tac_ruby/page_visibility.rb"),
        include_str!("tac_ruby/preferences.rb"),
        include_str!("tac_ruby/clipboard.rb"),
        include_str!("tac_ruby/share.rb"),
        include_str!("tac_ruby/notifications.rb"),
        include_str!("tac_ruby/geolocation.rb"),
        include_str!("tac_ruby/window_controls.rb")
    );
    pub(crate) const RUNTIME_PATH: &'static str = "ruby/tachyon-ruby.wasm";
    pub(crate) const GLUE_PATH: &'static str = "ruby/tachyon-ruby.js";
    pub(crate) const NOTICES_PATH: &'static str = "ruby/THIRD_PARTY_NOTICES.txt";
    const FAILURE_CODE: u16 = 1605;
    const MODELS: &'static str = "client/models";

    pub(crate) async fn runtime_files() -> Result<Vec<(String, Vec<u8>)>, Failure> {
        let runtime = RubyRuntimeBuild::runtime().await?;
        Ok(vec![
            (String::from(Self::RUNTIME_PATH), runtime),
            (
                String::from(Self::GLUE_PATH),
                RubyRuntimeSources::GLUE.as_bytes().to_vec(),
            ),
            (
                String::from(Self::NOTICES_PATH),
                RubyRuntimeSources::NOTICES.as_bytes().to_vec(),
            ),
        ])
    }

    pub(crate) fn program(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
    ) -> Result<String, Failure> {
        let mut program = String::from(Self::PRELUDE);
        program.push_str(&crate::route_types::WebEventSources::ruby());
        program.push_str(&Self::models_program(snapshot_root)?);
        for (instance, class, source) in services {
            program.push_str(&Self::service(snapshot_root, instance, class, source)?);
        }
        Ok(program)
    }

    pub(crate) fn models_program(snapshot_root: &Path) -> Result<String, Failure> {
        let models = Self::client_models(snapshot_root)?.join("\n");
        let shapes = Self::single_quoted(&Self::model_shapes(snapshot_root)?);
        Ok(format!("\n{models}\nTacValues.register_models({shapes})\n"))
    }

    fn model_shapes(snapshot_root: &Path) -> Result<String, Failure> {
        let table = crate::route_types::ModelTable::read_realm(snapshot_root, "client", "rb")
            .map_err(|message| Self::failure(&message))?;
        let mut described = serde_json::Map::new();
        for name in table.record_names() {
            table.shape_description(&crate::route_types::Shape::Model(name), &mut described);
        }
        Ok(serde_json::Value::Object(described).to_string())
    }

    pub(crate) fn client_models(snapshot_root: &Path) -> Result<Vec<String>, Failure> {
        let mut paths = Vec::new();
        Self::model_paths(&snapshot_root.join(Self::MODELS), 0, &mut paths);
        paths.sort();
        let mut models = Vec::new();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(|error| {
                Self::failure(&format!("Cannot read {}: {error}", path.display()))
            })?;
            models.push(String::from(crate::without_bom(&text)));
        }
        Ok(models)
    }

    fn model_paths(folder: &Path, depth: usize, paths: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.is_dir() && depth < 8 {
                Self::model_paths(&path, depth + 1, paths);
            } else if metadata.is_file() && path.extension().is_some_and(|value| value == "rb") {
                paths.push(path);
            }
        }
    }

    fn service(
        snapshot_root: &Path,
        instance: &str,
        class: &str,
        source: &Path,
    ) -> Result<String, Failure> {
        let text = std::fs::read_to_string(source).map_err(|error| {
            Self::failure(&format!("Cannot read {}: {error}", source.display()))
        })?;
        let text = crate::without_bom(&text);
        let identity = PathBuf::from(format!("client/services/{class}.rb"));
        let relative = source.strip_prefix(snapshot_root).unwrap_or(&identity);
        NeededServices::RUBY.refuse(snapshot_root, relative, text)?;
        let roles = crate::behavior::ServiceRoles::read(relative, text)?;
        let assembled =
            crate::annotation_modules::assemble(snapshot_root, relative, &roles.source, "tac")?
                .unwrap_or_else(|| roles.source.clone());
        let mut events = crate::events::extract(&identity, &assembled, "tac")?;
        if let Some(module) = events.as_mut() {
            crate::events::describe(module, relative, snapshot_root, "client")?;
        }
        let evented = events
            .as_ref()
            .map_or(assembled.as_str(), |module| module.source.as_str());
        let lowered = crate::model_sources::ModelSources::client_metadata(snapshot_root, "rb")?
            .lowered(&identity, evented, |text| {
                crate::behavior::lower(&identity, text, "tac")
            })?
            .unwrap_or_else(|| evented.to_owned());
        let code = crate::annotation_layout::masked_source(&identity, &roles.source)?;
        let members = RubyMembers::read(&code).without_needed(&roles.source);
        let mut metadata = Self::metadata(&members, events.as_ref(), &roles.required);
        let models = crate::route_types::ModelTable::read_realm(snapshot_root, "client", "rb")
            .map_err(|message| Self::failure(&message))?;
        metadata["readers"] =
            RubyModelReaders::new(models, &roles.source, &code).describe(&members);
        let instance = serde_json::to_string(instance).unwrap_or_default();
        Ok(format!(
            "\n{lowered}\nTacServices.register({instance}, {class}, {})\n",
            Self::single_quoted(&metadata.to_string())
        ))
    }

    pub(crate) fn metadata(
        members: &RubyMembers,
        events: Option<&EventModule>,
        required: &[String],
    ) -> serde_json::Value {
        let event_type = events.and_then(|module| module.types.first());
        let mut event_methods = Vec::new();
        let mut starts = Vec::new();
        let mut stops = Vec::new();
        let mut listeners = Vec::new();
        if let Some(event_type) = event_type {
            starts.extend(event_type.post_construct.iter().cloned());
            stops.extend(event_type.pre_destroy.iter().cloned());
            for listener in &event_type.listeners {
                event_methods.push(listener.method.clone());
                listeners.push(serde_json::json!({
                    "method": listener.method,
                    "events": listener.events,
                    "replay": listener.replay,
                    "order": listener.order,
                    "asynchronous": listener.asynchronous,
                    "condition": listener.condition,
                    "models": listener.models,
                    "returns": listener.returns,
                }));
            }
        }
        event_methods.extend(starts.iter().cloned());
        event_methods.extend(stops.iter().cloned());
        let methods: Vec<&String> = members
            .methods
            .iter()
            .filter(|name| !event_methods.contains(name))
            .collect();
        let mut names = serde_json::Map::new();
        for name in members.fields.iter().chain(methods.iter().copied()) {
            names.insert(
                Self::page_name(name),
                serde_json::Value::String(name.clone()),
            );
        }
        serde_json::json!({
            "fields": Self::page_names(&members.fields),
            "writable": Self::page_names(&members.writable),
            "methods": methods.iter().map(|name| Self::page_name(name)).collect::<Vec<_>>(),
            "names": names,
            "required": required,
            "starts": starts,
            "stops": stops,
            "listeners": listeners,
        })
    }

    fn page_names(names: &[String]) -> Vec<String> {
        names.iter().map(|name| Self::page_name(name)).collect()
    }

    fn page_name(name: &str) -> String {
        crate::companion::camel_case(name)
    }

    fn single_quoted(text: &str) -> String {
        format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            Self::FAILURE_CODE,
            message,
            Some(String::from(
                "Keep each Ruby service and model a readable UTF-8 file in its client folder.",
            )),
            None,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RubyWebServices;
    use std::path::{Path, PathBuf};

    mod browser_http_tests;
    mod needed_service_tests;

    fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, text).expect("source");
        path
    }

    #[test]
    fn a_service_registers_its_members_events_and_properties() {
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "client/models/Tally.rb",
            "# @!attribute [r] total\n#   @return [Integer]\nTally = Data.define(:total)\n",
        );
        write(
            root.path(),
            "client/models/CounterReset.rb",
            "# @!attribute [r] count\n#   @return [Integer]\nCounterReset = Data.define(:count)\n",
        );
        let service = write(
            root.path(),
            "client/services/CounterService.rb",
            "#[Service]\n#[NeedsAll]\nclass CounterService\n  attr_reader :count, :label\n\n  #[OnStart]\n  def start\n    @count = 0\n    @label = \"#{@label}!\"\n  end\n\n  def increment\n    @count += 1\n  end\n\n  # @param reset [CounterReset]\n  #[OnEvent]\n  def reset(reset)\n    @count = reset.count\n  end\nend\n",
        );
        let program = RubyWebServices::program(
            root.path(),
            &[(
                String::from("counter"),
                String::from("CounterService"),
                service,
            )],
        )
        .expect("program");
        assert!(program.starts_with(RubyWebServices::PRELUDE), "{program}");
        assert!(program.contains("Tally = Data.define(:total)"), "{program}");
        assert!(!program.contains("#[OnStart]"), "{program}");
        let registration = program
            .lines()
            .find(|line| line.starts_with("TacServices.register(\"counter\", CounterService, '"))
            .expect("registration");
        for expected in [
            "\"fields\":[\"count\",\"label\"]",
            "\"methods\":[\"increment\"]",
            "\"required\":[\"*\"]",
            "\"starts\":[\"start\"]",
            "\"method\":\"reset\"",
            "\"events\":[\"CounterReset\"]",
            "\"models\":{\"CounterReset\":",
        ] {
            assert!(
                registration.contains(expected),
                "{expected}: {registration}"
            );
        }
    }

    struct Page;

    impl Page {
        const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\nglobalThis.fetch = async (url) => new Response(await readFile(url))\n";

        const COUNTER: &str = "#[Service]\n#[Lifetime({\"value\":\"prototype\"})]\n#[NeedsAll]\nclass CounterService\n  attr_accessor :count\n  attr_reader :label\n\n  #[OnStart]\n  def start\n    @count = 0\n    @events = ApplicationEventPublisher.new\n    @label = \"#{@label} clicks\"\n    @events.publish_event(CounterStarted.new(label: @label))\n  end\n\n  def increment\n    @count += 1\n    @events.publish_event(CounterAdded.new(count: @count))\n    @count\n  end\n\n  def doubled\n    @count * 2\n  end\n\n  def add(by, why)\n    @count += by\n    \"#{why} #{@count}\"\n  end\nend\n";

        const COUNTER_DRIVER: &str = "const { default: CounterService, __tachyon_event_plan__: plan } = await import('./CounterService.js')\n\
             const events = []\n\
             const one = new CounterService({ label: 'many', ignored: 1 }), two = new CounterService({})\n\
             for (const [tag, owner] of [['one', one], ['two', two]]) {\n\
             \x20 Object.defineProperty(owner, '__tachyonPublisher', { value: { publishEvent: (event, value) => events.push([tag, event, value]) } })\n\
             \x20 owner.__tachyonStarted()\n\
             }\n\
             one.increment(); one.increment(); two.increment()\n\
             const doubled = one.doubled()\n\
             one.count = 40\n\
             const after = one.increment()\n\
             const label = one.label\n\
             const added = one.add(new Event('click'), 9, 'now')\n\
             let bigStart = '', bigCall = ''\n\
             try { new CounterService({ label: 'x'.repeat(1100000) }) } catch (error) { bigStart = error.message }\n\
             try { one.add(new Event('click'), 1, 'x'.repeat(70000)) } catch (error) { bigCall = error.message }\n\
             one.__tachyonDispose()\n\
             let stale = false\n\
             try { one.count } catch { stale = true }\n\
             console.log(JSON.stringify({ doubled, after, other: two.count, label, otherLabel: two.label, added, stale, events, keys: Object.keys(two), plan: plan.types[0], bigStart, bigCall }))\n\
             process.exit(0)\n";

        const SIGNAL: &str = "#[Service]\nclass SignalService\n  attr_accessor :large, :echoed\n\n  def initialize\n    @large = []\n    @echoed = 0\n    @events = ApplicationEventPublisher.new\n  end\n\n  def notify(size)\n    @events.publish_event(SignalSent.new(size: size))\n  end\n\n  # @param sent [SignalSent]\n  # @return [SignalEcho]\n  #[OnEvent({\"condition\":\"value.size > 5\"})]\n  def record(sent)\n    @large << sent.size\n    SignalEcho.new(value: sent.size * 10)\n  end\n\n  # @param reply [SignalEcho]\n  #[OnEvent]\n  #[Order(1)]\n  def echo(reply)\n    @echoed = reply.value\n  end\nend\n";

        const MODELS: [(&str, &str); 4] = [
            (
                "client/models/CounterStarted.rb",
                "# @!attribute [r] label\n#   @return [String]\nCounterStarted = Data.define(:label)\n",
            ),
            (
                "client/models/CounterAdded.rb",
                "# @!attribute [r] count\n#   @return [Integer]\nCounterAdded = Data.define(:count)\n",
            ),
            (
                "client/models/SignalSent.rb",
                "# @!attribute [r] size\n#   @return [Integer]\nSignalSent = Data.define(:size)\n",
            ),
            (
                "client/models/SignalEcho.rb",
                "# @!attribute [r] value\n#   @return [Integer]\nSignalEcho = Data.define(:value)\n",
            ),
        ];

        const SIGNAL_DRIVER: &str = "const { default: SignalService, __tachyon_event_plan__: plan } = await import('./SignalService.js')\n\
             const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
             const events = []\n\
             const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
             for (const event of ['SignalSent', 'SignalEcho']) bus.attach({ receive: (value) => events.push([event, value]) }, { listeners: [{ method: 'receive', event, order: -100 }] }, {})\n\
             const signal = new SignalService({})\n\
             bus.attach(signal, plan.types[0], { applicationEventPublisher: null })\n\
             await bus.initialize(signal)\n\
             signal.notify(3); signal.notify(8)\n\
             console.log(JSON.stringify({ large: signal.large, echoed: signal.echoed, events }))\n\
             process.exit(0)\n";

        const CONTRACT_DRIVER: &str = "const { TachyonRuby } = await import('./ruby/tachyon-ruby.js')\n\
             const module = await WebAssembly.compile(await readFile('./ruby/tachyon-ruby.wasm'))\n\
             let printed = ''\n\
             const runtime = await TachyonRuby.start(module, (line) => { printed += line }, () => {})\n\
             const failure = runtime.boot(await readFile('./program.rb', 'utf8'))\n\
             console.log(JSON.stringify({ printed, failure }))\n\
             process.exit(0)\n";

        const STORAGE_FILES: [(&str, &str); 2] = [
            (
                "client/setups/BrowserSetup.rb",
                "#[Setup]\nclass BrowserSetup\n  def self.configure(_execution)\n    {}\n  end\nend\n",
            ),
            (
                "client/entities/NoteEntity.rb",
                "#[Entity({\"adapter\":\"indexeddb\"})]\n#[Table(\"notes\")]\nclass NoteEntity\n  # @return [String]\n  #[Id]\n  attr_reader :id\nend\n",
            ),
        ];

        const NOTES: &str = "#[Service]\n#[CacheSettings({\"name\":\"notes\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})]\nclass NotesService\n  #[OnStart]\n  def start\n    @reads = 0\n    @behaviors = Behaviors.create(BrowserSetup, partition: \"notes\", signal: BehaviorSignal.new)\n  end\n\n  #[Cached]\n  def find(key)\n    @reads += 1\n    { \"key\" => key, \"reads\" => @reads }\n  end\n\n  #[ClearCache({\"method\":\"find\"})]\n  def forget(key)\n    nil\n  end\n\n  def save(id, text)\n    @behaviors.repository(NoteEntity).save({ \"id\" => id, \"text\" => text })\n  end\n\n  #[AllOrNothing({\"adapter\":\"indexeddb\"})]\n  def save_then_refuse(id)\n    save(id, \"draft\")\n    raise BehaviorError.new(\"validation\", \"the note was refused\")\n  end\n\n  def list\n    @behaviors.repository(NoteEntity).find({})\n  end\nend\n";

        const NOTES_DRIVER: &str = "console.info = () => {}\n\
             globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
             ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
             const { default: NotesService } = await import('./NotesService.js')\n\
             const notes = new NotesService({})\n\
             const first = notes.find('a')\n\
             const second = notes.find('a')\n\
             notes.forget('a')\n\
             const third = notes.find('a')\n\
             notes.save('n1', 'hello')\n\
             let refused = ''\n\
             try { notes.saveThenRefuse('n2') } catch (error) { refused = error.message }\n\
             const listed = notes.list()\n\
             notes.save('n9', 'x'.repeat(5000))\n\
             const long = notes.list().at(-1).text.length\n\
             console.log(JSON.stringify({ first, second, third, refused, listed, long }))\n\
             process.exit(0)\n";

        const PRICE_SETUP: &str = "#[Setup]\nclass PriceSetup\n  def self.configure(execution)\n    entries = {}\n    cache = {\n      get: ->(key, _execution) { entries.fetch(key, { \"hit\" => false }) },\n      set: ->(key, value, execution) { entries[key] = { \"hit\" => true, \"value\" => value, \"expiresAt\" => execution.expires_at } },\n      delete: ->(key, _execution) { entries.delete(key) }\n    }\n    { caches: { prices: cache }, authorize: ->(_call, _execution) { true } }\n  end\nend\n";

        const PRICES: &str = "#[Service]\n#[CacheSettings({\"name\":\"prices\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":16})]\nclass PriceService\n  attr_reader :reads, :attempts\n\n  #[OnStart]\n  def start\n    @reads = 0\n    @attempts = 0\n    @behaviors = Behaviors.create(PriceSetup, partition: \"prices\", signal: BehaviorSignal.new)\n  end\n\n  #[Cached]\n  def price(sku)\n    @reads += 1\n    sku.size * 10\n  end\n\n  #[Retry({\"maxRetries\":3,\"delayMs\":5,\"multiplier\":1,\"maxDelayMs\":5,\"jitterMs\":0})]\n  def steady\n    @attempts += 1\n    raise \"not yet\" if @attempts < 3\n    @attempts\n  end\nend\n";

        const PRICE_DRIVER: &str = "const { default: PriceService } = await import('./PriceService.js')\n\
             const prices = new PriceService({})\n\
             Object.defineProperty(prices, '__tachyonPublisher', { value: { publishEvent: () => {} } })\n\
             prices.__tachyonStarted()\n\
             const first = prices.price('anvil'), second = prices.price('anvil')\n\
             const retried = prices.steady()\n\
             console.log(JSON.stringify({ first, second, reads: prices.reads, retried }))\n\
             process.exit(0)\n";

        fn node_available() -> bool {
            let found = std::process::Command::new("node")
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success());
            assert!(
                found || std::env::var_os("CI").is_none(),
                "CI must provide node"
            );
            found
        }

        fn skip_reason() -> Option<String> {
            if !Self::node_available() {
                return Some(String::from("node is unavailable"));
            }
            Self::runtime_skip_reason()
        }

        fn runtime_skip_reason() -> Option<String> {
            super::RubyRuntimeBuild::unavailable_reason()
        }

        async fn publish(root: &Path, services: &[(&str, &str, &str)]) {
            for (relative, text) in Self::MODELS {
                write(root, relative, text);
            }
            let mut chosen = Vec::new();
            for (instance, class, source) in services {
                let path = write(root, &format!("client/services/{class}.rb"), source);
                chosen.push((
                    String::from(*instance),
                    String::from(*class),
                    crate::tac_web::WebLanguage::Ruby,
                    path,
                ));
            }
            let jobs = crate::tac_web::BuildJobs::new(1);
            let published =
                crate::tac_web::build_all(root, &chosen, &jobs, crate::BuildProfile::Release)
                    .await
                    .expect("build");
            for (name, bytes) in published.files {
                let path = root.join(name);
                std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
                std::fs::write(path, bytes).expect("publish");
            }
        }

        fn run(root: &Path, driver: &str) -> serde_json::Value {
            std::fs::write(
                root.join("drive.mjs"),
                format!("{}{driver}", Self::FILE_FETCH),
            )
            .expect("driver");
            let output = std::process::Command::new("node")
                .arg("drive.mjs")
                .current_dir(root)
                .output()
                .expect("node");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success(),
                "{stdout}{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let last = stdout.lines().last().unwrap_or_default();
            serde_json::from_str(last).expect("report")
        }
    }

    #[tokio::test]
    async fn a_ruby_service_answers_the_page_from_the_mruby_runtime() {
        if let Some(reason) = Page::skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        Page::publish(root.path(), &[("counter", "CounterService", Page::COUNTER)]).await;
        let report = Page::run(root.path(), Page::COUNTER_DRIVER);
        assert_eq!(report["doubled"], 4);
        assert_eq!(report["after"], 41);
        assert_eq!(report["other"], 1);
        assert_eq!(report["label"], "many clicks");
        assert_eq!(report["otherLabel"], " clicks");
        assert_eq!(report["added"], "now 50");
        assert_eq!(report["stale"], true);
        assert_eq!(report["keys"], serde_json::json!(["count", "label"]));
        assert_eq!(
            report["events"],
            serde_json::json!([
                ["one", "CounterStarted", {"label": "many clicks"}],
                ["two", "CounterStarted", {"label": " clicks"}],
                ["one", "CounterAdded", {"count": 1}],
                ["one", "CounterAdded", {"count": 2}],
                ["two", "CounterAdded", {"count": 1}],
                ["one", "CounterAdded", {"count": 41}],
            ])
        );
        assert_eq!(
            report["bigStart"],
            "Web service counter: The request is 1100115 bytes. The limit is 1048576 bytes. Pass less data to the service."
        );
        assert_eq!(
            report["bigCall"],
            "Web service counter: The request to add is 70121 bytes. The limit is 65536 bytes. Pass less data to the service."
        );
        assert!(root.path().join("ruby/tachyon-ruby.wasm").is_file());
    }

    #[tokio::test]
    async fn ruby_listeners_follow_their_conditions_and_order() {
        if let Some(reason) = Page::skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        Page::publish(root.path(), &[("signal", "SignalService", Page::SIGNAL)]).await;
        std::fs::write(
            root.path().join("application-events.mjs"),
            concat!(
                include_str!("template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        )
        .expect("event runtime");
        let report = Page::run(root.path(), Page::SIGNAL_DRIVER);
        assert_eq!(report["large"], serde_json::json!([8]));
        assert_eq!(report["echoed"], 80);
        assert_eq!(
            report["events"],
            serde_json::json!([
                ["SignalSent", {"size": 3}],
                ["SignalSent", {"size": 8}],
                ["SignalEcho", {"value": 80}],
            ])
        );
    }

    #[tokio::test]
    async fn the_behavior_runtime_contract_passes_on_mruby() {
        if let Some(reason) = Page::skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        for (name, bytes) in RubyWebServices::runtime_files().await.expect("runtime") {
            let path = root.path().join(&name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("runtime");
        }
        let program = format!(
            "{}\n{}\n{}",
            RubyWebServices::PRELUDE,
            include_str!("behavior/polyglot/ruby_runtime.rb"),
            include_str!("../tests/fixtures/behavior/ruby_runtime_contract.rb")
        );
        std::fs::write(root.path().join("program.rb"), program).expect("program");
        let report = Page::run(root.path(), Page::CONTRACT_DRIVER);
        assert_eq!(report["failure"], serde_json::Value::Null, "{report}");
        assert!(
            report["printed"]
                .as_str()
                .is_some_and(|printed| printed.contains("Ruby behavior boundary contracts passed")),
            "{report}"
        );
    }

    #[tokio::test]
    async fn a_cached_ruby_service_answers_the_second_call_from_its_cache() {
        if let Some(reason) = Page::skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "client/setups/PriceSetup.rb",
            Page::PRICE_SETUP,
        );
        Page::publish(root.path(), &[("prices", "PriceService", Page::PRICES)]).await;
        let report = Page::run(root.path(), Page::PRICE_DRIVER);
        assert_eq!(report["first"], 50, "{report}");
        assert_eq!(report["second"], 50, "{report}");
        assert_eq!(report["reads"], 1, "{report}");
        assert_eq!(report["retried"], 3, "{report}");
    }

    #[tokio::test]
    async fn a_ruby_service_reads_the_builtin_cache_and_store_synchronously() {
        if let Some(reason) = Page::skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        for (relative, text) in Page::STORAGE_FILES {
            write(root.path(), relative, text);
        }
        write(
            root.path(),
            "tac-storage.js",
            include_str!("template/tac-storage.js"),
        );
        Page::publish(root.path(), &[("notes", "NotesService", Page::NOTES)]).await;
        let report = Page::run(root.path(), Page::NOTES_DRIVER);
        assert_eq!(report["first"], serde_json::json!({"key": "a", "reads": 1}));
        assert_eq!(report["second"], report["first"]);
        assert_eq!(report["third"], serde_json::json!({"key": "a", "reads": 2}));
        assert!(
            report["refused"]
                .as_str()
                .is_some_and(|message| message.contains("the note was refused")),
            "{report}"
        );
        assert_eq!(
            report["listed"],
            serde_json::json!([{"id": "n1", "text": "hello"}])
        );
        assert_eq!(report["long"], 5000, "{report}");
    }

    #[tokio::test]
    async fn the_runtime_ships_under_the_precache_budget() {
        if let Some(reason) = Page::runtime_skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let files = RubyWebServices::runtime_files().await.expect("runtime");
        let runtime = &files[0].1;
        assert!(runtime.starts_with(b"\0asm"));
        assert!(runtime.len() < 4 * 1024 * 1024);
        assert_eq!(files[1].0, "ruby/tachyon-ruby.js");
    }

    #[tokio::test]
    async fn the_runtime_is_built_once_into_the_user_cache_and_never_into_the_project() {
        if let Some(reason) = Page::runtime_skip_reason() {
            eprintln!("skipping: {reason}");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let source = write(
            root.path(),
            "client/services/CounterService.rb",
            Page::COUNTER,
        );
        let chosen = [(
            String::from("counter"),
            String::from("CounterService"),
            crate::tac_web::WebLanguage::Ruby,
            source,
        )];
        let jobs = crate::tac_web::BuildJobs::new(1);
        let published =
            crate::tac_web::build_all(root.path(), &chosen, &jobs, crate::BuildProfile::Release)
                .await
                .expect("build");
        let cached = super::RubyRuntimeBuild::cached_path();
        assert!(
            cached.starts_with(crate::cache::root()),
            "{}",
            cached.display()
        );
        let runtime = published
            .files
            .iter()
            .find(|(name, _)| name == "ruby/tachyon-ruby.wasm")
            .expect("published runtime");
        assert_eq!(runtime.1, std::fs::read(&cached).expect("cached"));
        let mut project_files = Vec::new();
        for entry in std::fs::read_dir(root.path()).expect("project") {
            project_files.push(entry.expect("entry").file_name());
        }
        assert_eq!(project_files, ["client"]);
    }
}
