import { cpSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export class PageServicesFixture {
  static features = fileURLToPath(new URL('../../website/content/features', import.meta.url))
  static tallySteps = [
    ['wait', ['0 in the basket', 'Add two']],
    ['click', 'Add two'],
    ['wait', ['2 in the basket', '2 items']],
  ]
  static reportSteps = [
    ['wait', ['Report', 'Rename']],
    ['click', 'Rename'],
    ['wait', ['Quarterly report']],
    ['click', 'Resize'],
    ['size', [1280, 800]],
  ]
  static pickerStatuses = { js: ['read', 'cancelled'], rs: ['read', 'cancelled'], swift: ['read', 'cancelled'], kt: ['read', 'cancelled'], cs: ['Read', 'Cancelled'] }

  static storedFieldSteps = [
    ['wait', ['Visits 0', 'Page fresh']],
    ['click', 'Visit'],
    ['wait', ['Visits 1']],
    ['click', 'Mark the page'],
    ['wait', ['Page marked']],
    ['click', 'Reload the page'],
    ['wait', ['Page fresh', 'Visits 1']],
    ['click', 'Visit'],
    ['wait', ['Visits 2']],
  ]
  static storedFieldFiles = {
    'client/pages/tac.html': '<visit-panel hydrate="load" />\n',
    'client/components/visit/panel/tac.html': '<button on:click="visit.visit()">Visit</button>\n<p>Visits {visit.visits}</p>\n<button on:click="reload.mark()">Mark the page</button>\n<p>Page {reload.state}</p>\n<button on:click="reload.again()">Reload the page</button>\n',
    'client/services/VisitService.rs': `#[Service]
#[derive(Default)]
struct VisitService {
    #[LocalState]
    visits: i64,
}

impl VisitService {
    pub fn visit(&mut self) -> i64 {
        self.visits += 1;
        self.visits
    }
}
`,
    'client/services/ReloadService.js': `@Service
export default class ReloadService {
  state = 'fresh';

  mark() {
    this.state = 'marked';
  }

  again() {
    location.reload();
  }
}
`,
  }

  /** @param {string} name @param {string} fallback */
  static list(name, fallback) {
    return (process.env[name] ?? fallback).split(',').map(item => item.trim()).filter(Boolean)
  }

  /** @param {string} project @param {string} feature @param {string} name @param {string} id @param {Record<string, unknown>} [settings] */
  static stage(project, feature, name, id, settings = {}) {
    const source = path.join(this.features, feature, 'client')
    for (const entry of ['pages', 'components']) cpSync(path.join(source, entry), path.join(project, 'client', entry), { recursive: true })
    const manifest = JSON.parse(readFileSync(path.join(source, 'manifest.json'), 'utf8'))
    manifest.tac = { ...manifest.tac, ...settings, application: { ...manifest.tac?.application, id } }
    manifest.name ??= name
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest, null, 2))
    return manifest.name.replace(/[^A-Za-z0-9]/g, '')
  }

  /** @param {string} project @param {string} feature @param {string} file */
  static copyService(project, feature, file) {
    const target = path.join(project, 'client/services', path.basename(file))
    mkdirSync(path.dirname(target), { recursive: true })
    cpSync(path.join(this.features, feature, file), target)
  }

  /** @param {string} project @param {string} language @param {Record<string, unknown>} [settings] */
  static stageTally(project, language, settings = {}) {
    const application = this.stage(project, 'web-assembly-services', 'Tally', `dev.tachyon.tally-${language}`, settings)
    this.copyService(project, 'web-assembly-services', `client/services/TallyService.${language}`)
    return application
  }

  /** @param {string} project @param {string} language */
  static stagePicker(project, language) {
    const application = this.stage(project, 'native-capabilities', 'Picker', `dev.tachyon.picker-${language}`)
    this.copyService(project, 'native-capabilities', `client/services/PickerService.${language}`)
    const notes = path.join(project, 'notes.txt')
    writeFileSync(notes, 'Quarterly notes\n')
    return { application, notes }
  }

  /** @param {string} project */
  static stageStoredField(project) {
    for (const [name, text] of Object.entries(this.storedFieldFiles)) {
      mkdirSync(path.dirname(path.join(project, name)), { recursive: true })
      writeFileSync(path.join(project, name), text)
    }
    const visibility = Object.fromEntries(['visibility', 'constants', 'cohesion'].map((name) => [name, { check: false }]))
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify({ name: 'Stored Field', start_url: '/', tac: { ...visibility, application: { id: 'dev.tachyon.stored-field' } } }, null, 2))
    return 'StoredField'
  }

  /** @param {string} project @param {string} language @param {string} [folder] */
  static stageWindowCaller(project, language, folder = 'client') {
    const application = this.stage(project, 'window-controls', 'Catalog', `dev.tachyon.window-${language}`)
    this.copyService(project, 'window-controls', `${folder}/services/ReportService.${language}`)
    return application
  }
}
