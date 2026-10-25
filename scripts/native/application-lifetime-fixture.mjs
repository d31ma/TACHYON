import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

export class ApplicationLifetimeFixture {
  static applicationId = 'dev.tachyon.lifetime-gate';
  static androidPackage = 'dev.tachyon.lifetime_gate';
  static name = 'LifetimeGate';
  static menus = {
    file: [
      { id: 'status', label: 'Status', route: '/status', shortcut: { key: 'k', modifiers: ['command'] } },
      { id: 'refresh', label: 'Refresh', action: { service: 'LifetimeService', method: 'refresh' }, shortcut: { key: '2', modifiers: ['command'] } },
    ],
    view: [
      { id: 'show-running', label: "What's Running", action: { service: 'LifetimeService', method: 'show_running', event: 'SectionChosen' }, shortcut: { key: '1', modifiers: ['command'] } },
    ],
    help: [{ id: 'guide', label: 'Guide', route: '/guide', shortcut: { key: '3', modifiers: ['command'] } }],
  };

  static application = `use crate::models::SectionChosen;

#[Service]
#[Lifetime({"value":"application"})]
struct LifetimeService {
    page_events: CurrentPageEvents,
    instance: String,
    visits: std::sync::atomic::AtomicI64,
}

impl LifetimeService {
    fn new(page_events: CurrentPageEvents) -> Self {
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let instance = format!("{}-{started}", std::process::id());
        Self::note(&format!("created {instance}"));
        Self { page_events, instance, visits: std::sync::atomic::AtomicI64::new(0) }
    }

    #[OnStop]
    pub fn stopped(&self) {
        Self::note(&format!("stopped {}", self.instance));
    }

    pub fn instance(&self) -> String {
        self.instance.clone()
    }

    pub fn visit(&self) -> i64 {
        let visits = self.visits.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Self::note(&format!("visit {} {visits}", self.instance));
        visits
    }

    pub fn heard(&self, section: &str) {
        Self::note(&format!("heard {section}"));
    }

    pub fn left(&self, visit: i64) {
        Self::note(&format!("left {} {visit}", self.instance));
    }

    pub fn show_running(&self) {
        let delivered = self.page_events.publish_event(SectionChosen { section: String::from("running") });
        Self::note(if delivered { "forwarded" } else { "unheard" });
    }

    pub fn refresh(&self) {
        Self::note(&format!("refreshed {}", self.instance));
    }

    fn journal() -> std::path::PathBuf {
        #[cfg(target_os = "android")]
        let folder = std::path::PathBuf::from("/data/data/${ApplicationLifetimeFixture.androidPackage}/files");
        #[cfg(not(target_os = "android"))]
        let folder = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Documents");
        let _ = std::fs::create_dir_all(&folder);
        folder.join("lifetime.txt")
    }

    fn note(line: &str) {
        use std::io::Write as _;
        let file = std::fs::OpenOptions::new().create(true).append(true).open(Self::journal());
        let _ = file.map(|mut file| writeln!(file, "{line}"));
    }
}
`;

  static page = `use crate::models::SectionChosen;

#[Service]
struct SectionService {
    #[Needs]
    lifetime: std::sync::Arc<LifetimeService>,
    pub section: String,
    pub instance: String,
    pub visits: i64,
}

impl SectionService {
    #[OnStart]
    fn started(&mut self) {
        self.section = String::from("none");
        self.instance = self.lifetime.instance();
        self.visits = self.lifetime.visit();
    }

    #[OnStop]
    fn stopped(&mut self) {
        self.lifetime.left(self.visits);
    }

    #[OnEvent]
    fn chosen(&mut self, chosen: SectionChosen) {
        self.section = chosen.section;
        self.lifetime.heard(&self.section);
    }
}
`;

  static files() {
    const checks = { names: { check: false }, constants: { check: false }, visibility: { check: false }, duplication: { check: false }, cohesion: { check: false } };
    const { applicationId, name, menus } = ApplicationLifetimeFixture;
    const files = {
      'client/manifest.json': JSON.stringify({ name, start_url: '/', tac: { application: { id: applicationId, version: '0.0.1', build: 1, menus }, ...checks } }, null, 2),
      'client/models/SectionChosen.rs': 'pub struct SectionChosen {\n    pub section: String,\n}\n',
      'client/services/menus/LifetimeService.rs': ApplicationLifetimeFixture.application,
      'client/services/SectionService.rs': ApplicationLifetimeFixture.page,
      'client/pages/guide/tac.html': '<gate-guide />',
      'client/components/gate/guide/tac.html': '<main><h1>Guide</h1></main>',
    };
    for (const [prefix, page, title] of [['', 'home', 'Home'], ['status/', 'status', 'Status']]) {
      files['client/pages/' + prefix + 'tac.html'] = `<gate-${page} hydrate="load" />`;
      files['client/components/gate/' + page + '/tac.html'] = `<main><h1>${title}</h1><p id="instance">{section.instance}</p><p id="visits">{section.visits}</p><p id="section">{section.section}</p></main>`;
    }
    return files;
  }

  static write(project) {
    for (const [relative, source] of Object.entries(ApplicationLifetimeFixture.files())) {
      const file = path.join(project, relative);
      mkdirSync(path.dirname(file), { recursive: true });
      writeFileSync(file, source);
    }
  }

  static journal(text) {
    const lines = text.split('\n').filter(Boolean);
    const created = lines.filter(line => line.startsWith('created ')).map(line => line.slice('created '.length));
    const visits = lines.filter(line => line.startsWith('visit ')).map(line => line.split(' '));
    const left = lines.filter(line => line.startsWith('left ')).map(line => line.split(' '));
    return { lines, created, visits, left };
  }
}
