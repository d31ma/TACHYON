import { AnnotationReach } from './annotation-reach.mjs'
import { SiteContent } from './site-content.mjs'
import { languageOf } from '../client/shared/scripts/platforms.js'

/** @typedef {import('./rule-table-parser.mjs').Realm} Realm */
/** @typedef {import('./snippet-units.mjs').SnippetUnit} SnippetUnit */
/** @typedef {import('./snippet-units.mjs').UnitFile} UnitFile */
/** @typedef {UnitFile & { realm: Realm }} RealmFile */
/** @typedef {{ language: string, kind: string, reason: string, code?: string, stated?: boolean }} Gap */
/** @typedef {{ realm: Realm, present: string[], gaps: Gap[] }} RealmCoverage */
/** @typedef {{ page: string, id: string, title: string, realms: RealmCoverage[] }} UnitCoverage */
/** @typedef {{ unit: SnippetUnit, realm: Realm, language: string, annotations: string[], present: string[], shown: Map<SnippetUnit, Record<Realm, string[]>> }} GapQuestion */
/** @typedef {{ tac: { files: { name: string }[] }, yon: { entries: { files: { name: string }[] }[] } }} LanguagesPage */

export class CoverageAudit {
  static kinds = { missing: 'missing example', unsupported: 'not supported yet', notApplicable: 'not applicable' }
  static realms = /** @type {Realm[]} */ (['tac', 'yon'])
  static pathRealms = /** @type {[string, Realm][]} */ ([['client/', 'tac'], ['server/', 'yon'], ['middleware.', 'yon']])
  static controllerPrefix = 'server/controllers/'
  static viewExtensions = ['html', 'css']
  static modelFolder = '/models/'
  static codeRealms = /** @type {[RegExp, Realm][]} */ ([
    [/(?:@|#\[|\[<?|\/\/ @)\s*(?:RequestRoute|request_route|SocketRoute|socket_route)\b/, 'yon'],
    [/(?:@|#\[|\[<?|\/\/ @)\s*Service\b/, 'tac'],
  ])
  static snakeCaseLanguage = 'py'
  static pascalCase = /^[A-Z]/
  static annotationMarker = /(?:@|#\[|\[<|^\s*\[)\s*([A-Za-z_]\w*)/gm
  static noExample = 'no example yet; the rule table accepts the annotations the other tabs use'
  static unknownToRuleTable = 'no example yet; the rule table does not check this language, so no refusal is known'
  static statedNotApplicable = 'the text says why this language does not apply'

  /** @type {Record<Realm, string[]>} */
  realmLanguages
  /** @type {AnnotationReach} */
  reach

  /**
   * @param {Record<Realm, string[]>} realmLanguages
   * @param {AnnotationReach} reach
   */
  constructor(realmLanguages, reach) {
    this.realmLanguages = realmLanguages
    this.reach = reach
  }

  /** @returns {CoverageAudit} */
  static load() {
    return new CoverageAudit(CoverageAudit.siteLanguages(SiteContent.data('languages.json')), AnnotationReach.load())
  }

  /**
   * @param {LanguagesPage} page
   * @returns {Record<Realm, string[]>}
   */
  static siteLanguages(page) {
    const tacNames = page.tac.files.map(file => file.name)
    const yonNames = page.yon.entries.flatMap(entry => entry.files.map(file => file.name)).filter(name => name.startsWith(CoverageAudit.controllerPrefix))
    return { tac: CoverageAudit.sourceExtensions(tacNames), yon: CoverageAudit.sourceExtensions(yonNames) }
  }

  /**
   * @param {string[]} names
   * @returns {string[]}
   */
  static sourceExtensions(names) {
    const extensions = names.map(name => name.split('.').pop() ?? '')
    return [...new Set(extensions)].filter(extension => !CoverageAudit.viewExtensions.includes(extension))
  }

  /**
   * @param {string} language
   * @returns {string}
   */
  static languageName(language) {
    return languageOf(`file.${language}`) ?? language
  }

  /**
   * @param {string} text
   * @param {string} language
   * @returns {boolean}
   */
  static namesLanguage(text, language) {
    const name = CoverageAudit.languageName(language).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    return new RegExp(`(?<![\\w#])${name}(?![\\w#])`).test(text)
  }

  /**
   * @param {SnippetUnit[]} units
   * @returns {UnitCoverage[]}
   */
  audit(units) {
    const shown = new Map(units.map(unit => [unit, this.shownLanguages(unit)]))
    return units.map(unit => this.unitCoverage(unit, shown))
  }

  /**
   * @param {SnippetUnit} unit
   * @returns {Record<Realm, string[]>}
   */
  shownLanguages(unit) {
    const files = this.sourceFiles(unit)
    return {
      tac: CoverageAudit.presentLanguages(files.filter(file => file.realm === 'tac')),
      yon: CoverageAudit.presentLanguages(files.filter(file => file.realm === 'yon')),
    }
  }

  /**
   * @param {SnippetUnit} unit
   * @param {Map<SnippetUnit, Record<Realm, string[]>>} shown
   * @returns {UnitCoverage}
   */
  unitCoverage(unit, shown) {
    const files = this.sourceFiles(unit)
    const realms = CoverageAudit.realms.filter(realm => files.some(file => file.realm === realm))
    const coverages = realms.map(realm => this.realmCoverage(unit, realm, files, shown))
    return { page: unit.page, id: unit.id, title: unit.title, realms: coverages }
  }

  /**
   * @param {SnippetUnit} unit
   * @returns {RealmFile[]}
   */
  sourceFiles(unit) {
    const placed = unit.files.map(file => ({ ...file, realm: CoverageAudit.realmOf(file) }))
    return /** @type {RealmFile[]} */ (placed.filter(file => this.isSource(file)))
  }

  /**
   * @param {UnitFile & { realm: Realm | undefined }} file
   * @returns {boolean}
   */
  isSource(file) {
    if (!file.realm) return false
    return this.realmLanguages[file.realm].includes(file.extension)
  }

  /**
   * @param {UnitFile} file
   * @returns {Realm | undefined}
   */
  static realmOf(file) {
    return file.named ? CoverageAudit.pathRealm(file.name) : CoverageAudit.guessedRealm(file.code)
  }

  /**
   * @param {string} name
   * @returns {Realm | undefined}
   */
  static pathRealm(name) {
    return CoverageAudit.pathRealms.find(([prefix]) => name.startsWith(prefix))?.[1]
  }

  /**
   * @param {string} code
   * @returns {Realm | undefined}
   */
  static guessedRealm(code) {
    return CoverageAudit.codeRealms.find(([marker]) => marker.test(code))?.[1]
  }

  /**
   * @param {RealmFile[]} files
   * @returns {string[]}
   */
  static presentLanguages(files) {
    const roleFiles = files.filter(file => !file.name.includes(CoverageAudit.modelFolder))
    const counted = roleFiles.length > 0 ? roleFiles : files
    return [...new Set(counted.map(file => file.extension))]
  }

  /**
   * @param {RealmFile[]} files
   * @returns {string[]}
   */
  static annotationsIn(files) {
    return [...new Set(files.flatMap(file => CoverageAudit.annotationsInFile(file)))]
  }

  /**
   * @param {RealmFile} file
   * @returns {string[]}
   */
  static annotationsInFile(file) {
    const names = [...file.code.matchAll(CoverageAudit.annotationMarker)].map(match => match[1])
    if (file.extension === CoverageAudit.snakeCaseLanguage) return names
    return names.filter(name => CoverageAudit.pascalCase.test(name))
  }

  /**
   * @param {SnippetUnit} unit
   * @param {Realm} realm
   * @param {RealmFile[]} files
   * @param {Map<SnippetUnit, Record<Realm, string[]>>} shown
   * @returns {RealmCoverage}
   */
  realmCoverage(unit, realm, files, shown) {
    const present = shown.get(unit)?.[realm] ?? []
    const annotations = CoverageAudit.annotationsIn(files.filter(file => file.realm === realm))
    const absent = this.realmLanguages[realm].filter(language => !present.includes(language))
    const gaps = absent.map(language => this.gap({ unit, realm, language, annotations, present, shown }))
    return { realm, present, gaps }
  }

  /**
   * @param {GapQuestion} question
   * @returns {Gap}
   */
  gap(question) {
    if (CoverageAudit.isStatedNotApplicable(question)) return CoverageAudit.notApplicable(question.language)
    const sibling = this.siblingShowing(question)
    if (sibling) return CoverageAudit.shownElsewhere(question.language, sibling)
    const refusal = this.reach.refusal(question.realm, question.language, question.annotations, question.present)
    if (refusal) return CoverageAudit.refused(question, refusal)
    return this.missing(question)
  }

  /**
   * @param {GapQuestion} question
   * @returns {boolean}
   */
  static isStatedNotApplicable(question) {
    const { unit, language } = question
    return (unit.notApplicable ?? []).includes(language) && CoverageAudit.namesLanguage(unit.text, language)
  }

  /**
   * @param {string} language
   * @returns {Gap}
   */
  static notApplicable(language) {
    return { language, kind: CoverageAudit.kinds.notApplicable, reason: CoverageAudit.statedNotApplicable, stated: true }
  }

  /**
   * @param {GapQuestion} question
   * @returns {SnippetUnit | undefined}
   */
  siblingShowing(question) {
    const { unit, realm, language, shown } = question
    if (!this.isScopedAway(unit.title, language)) return undefined
    const siblings = [...shown.keys()].filter(other => other !== unit && other.page === unit.page && other.id === unit.id)
    const inRealm = siblings.filter(other => (shown.get(other)?.[realm].length ?? 0) > 0)
    return inRealm.find(other => shown.get(other)?.[realm].includes(language)) ?? inRealm.find(other => !this.namesSomeLanguage(other.title))
  }

  /**
   * @param {string} title
   * @param {string} language
   * @returns {boolean}
   */
  isScopedAway(title, language) {
    if (CoverageAudit.namesLanguage(title, language)) return false
    return this.namesSomeLanguage(title)
  }

  /**
   * @param {string} title
   * @returns {boolean}
   */
  namesSomeLanguage(title) {
    const languages = new Set([...this.realmLanguages.tac, ...this.realmLanguages.yon])
    return [...languages].some(language => CoverageAudit.namesLanguage(title, language))
  }

  /**
   * @param {string} language
   * @param {SnippetUnit} sibling
   * @returns {Gap}
   */
  static shownElsewhere(language, sibling) {
    return { language, kind: CoverageAudit.kinds.notApplicable, reason: `the heading names other languages; "${sibling.title}" in the same topic covers this one` }
  }

  /**
   * @param {GapQuestion} question
   * @param {import('./annotation-reach.mjs').Refusal} refusal
   * @returns {Gap}
   */
  static refused(question, refusal) {
    return {
      language: question.language,
      kind: CoverageAudit.kinds.unsupported,
      reason: `${refusal.annotation} is refused in this language`,
      code: refusal.code,
      stated: CoverageAudit.namesLanguage(question.unit.text, question.language),
    }
  }

  /**
   * @param {GapQuestion} question
   * @returns {Gap}
   */
  missing(question) {
    const known = this.reach.realmLanguages[question.realm].includes(question.language)
    return { language: question.language, kind: CoverageAudit.kinds.missing, reason: known ? CoverageAudit.noExample : CoverageAudit.unknownToRuleTable }
  }
}
