import { mkdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'

export class MobileAccessibilityFixture {
  static NAME = 'AccessibilityAudit'
  static HEADING = 'Accessibility audit'
  static FIELD = 'Note'
  static BUTTON = 'Save note'
  static LINK = 'About this app'
  static IMAGE = 'Tachyon mark'
  static IDLE = 'Nothing saved'
  static TYPED = 'hello'
  static SAVED = 'Saved hello'
  static ABOUT_HEADING = 'About the audit'

  static files(applicationId) {
    const fixture = MobileAccessibilityFixture
    const checks = { names: { check: false }, constants: { check: false }, visibility: { check: false }, duplication: { check: false }, cohesion: { check: false } }
    return {
      'client/manifest.json': JSON.stringify({ name: fixture.NAME, start_url: '/', tac: { application: { id: applicationId, version: '0.0.1', build: 1 }, styles: ['/shared/audit.css'], ...checks } }),
      'client/shared/audit.css': 'html { font: -apple-system-body; font-family: system-ui, sans-serif; color: #111111; background: #ffffff; }\nbody { margin: 0; }\n',
      'client/components/audit/home/tac.css': `main { padding: 16px; }
label, input, button, a { display: block; box-sizing: border-box; min-height: 48px; min-width: 48px; font: inherit; }
label { line-height: 48px; }
input { width: 100%; padding: 8px; border: 2px solid #333333; }
button { margin: 16px 0; padding: 8px 16px; color: #ffffff; background: #0b4f9c; border: 0; }
a { line-height: 48px; color: #0b4f9c; }
svg { width: 48px; height: 48px; }
`,
      'client/pages/tac.html': '<audit-home />\n',
      'client/pages/about/tac.html': '<audit-about />\n',
      'client/components/audit/home/tac.html': `<main>
<h1>${fixture.HEADING}</h1>
<svg role="img" aria-label="${fixture.IMAGE}" viewBox="0 0 48 48"><circle cx="24" cy="24" r="20" fill="#0b4f9c" /></svg>
<label for="note">${fixture.FIELD}</label>
<input id="note" type="text" :value="auditNote.text" on:input="auditNote.text = $event.target.value">
<button id="save" type="button" on:click="auditNote.save()">${fixture.BUTTON}</button>
<p id="status" role="status">{auditNote.status}</p>
<a id="about" href="/about/">${fixture.LINK}</a>
</main>
`,
      'client/components/audit/about/tac.html': `<main><h1>${fixture.ABOUT_HEADING}</h1></main>\n`,
      'client/services/AuditNoteService.js': `@Service
export default class AuditNoteService {
  static IDLE = ${JSON.stringify(fixture.IDLE)};
  static SAVED = 'Saved ';
  text = '';
  status = AuditNoteService.IDLE;

  save() {
    this.status = AuditNoteService.SAVED + this.text;
  }
}
`,
    }
  }

  static write(project, applicationId) {
    for (const [relative, source] of Object.entries(MobileAccessibilityFixture.files(applicationId))) {
      const file = path.join(project, relative)
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, source)
    }
  }
}
