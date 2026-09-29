import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { RuleTableParser } from './rule-table-parser.mjs'

/** @typedef {import('./rule-table-parser.mjs').Realm} Realm */
/** @typedef {import('./rule-table-parser.mjs').AnnotationRule} AnnotationRule */
/** @typedef {{ annotation: string, code: string }} Refusal */

export class AnnotationReach {
  static ruleTable = fileURLToPath(new URL('../../crates/tachyon-core/src/annotation_rules.rs', import.meta.url))
  static codePrefix = 'TY'

  /** @type {Record<Realm, string[]>} */
  realmLanguages
  /** @type {Map<string, AnnotationRule>} */
  rules

  /**
   * @param {Record<Realm, string[]>} realmLanguages the languages whose annotations the build checks, per realm
   * @param {AnnotationRule[]} rules
   */
  constructor(realmLanguages, rules) {
    this.realmLanguages = realmLanguages
    this.rules = new Map(rules.map(rule => [AnnotationReach.plainName(rule.name), rule]))
  }

  /** @returns {AnnotationReach} */
  static load() {
    return AnnotationReach.parse(readFileSync(AnnotationReach.ruleTable, 'utf8'))
  }

  /**
   * @param {string} source the text of annotation_rules.rs
   * @returns {AnnotationReach}
   */
  static parse(source) {
    const table = RuleTableParser.parse(source)
    return new AnnotationReach(table.realmLanguages, table.rules)
  }

  /**
   * @param {string} name an annotation in any spelling, such as OnStart, on_start or onStart
   * @returns {string}
   */
  static plainName(name) {
    return name.replaceAll('_', '').toLowerCase()
  }

  /**
   * @param {string} name
   * @returns {AnnotationRule | undefined}
   */
  rule(name) {
    return this.rules.get(AnnotationReach.plainName(name))
  }

  /**
   * @param {Realm} realm
   * @param {string} language a file extension, such as rs
   * @param {string[]} annotations the annotation names that the other languages' examples use
   * @param {string[]} shownLanguages the languages that already have an example
   * @returns {Refusal | undefined}
   */
  refusal(realm, language, annotations, shownLanguages) {
    if (!this.realmLanguages[realm].includes(language)) return undefined
    const needed = this.knownRules(annotations).filter(rule => this.acceptsEvery(rule, realm, shownLanguages))
    const refused = needed.find(rule => !rule[realm].includes(language))
    return refused && { annotation: refused.name, code: `${AnnotationReach.codePrefix}${refused.code}` }
  }

  /**
   * @param {AnnotationRule} rule
   * @param {Realm} realm
   * @param {string[]} languages
   * @returns {boolean}
   */
  acceptsEvery(rule, realm, languages) {
    const checked = languages.filter(language => this.realmLanguages[realm].includes(language))
    return checked.every(language => rule[realm].includes(language))
  }

  /**
   * @param {string[]} annotations
   * @returns {AnnotationRule[]}
   */
  knownRules(annotations) {
    return annotations.map(name => this.rule(name)).filter(rule => rule !== undefined)
  }
}
