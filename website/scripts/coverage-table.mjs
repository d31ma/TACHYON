import { CoverageAudit } from './coverage-audit.mjs'

/** @typedef {import('./rule-table-parser.mjs').Realm} Realm */
/** @typedef {import('./coverage-audit.mjs').Gap} Gap */
/** @typedef {import('./coverage-audit.mjs').UnitCoverage} UnitCoverage */
/** @typedef {import('./coverage-audit.mjs').RealmCoverage} RealmCoverage */
/** @typedef {{ present: number, missing: number, unsupported: number, unstated: number, notApplicable: number }} Counts */
/** @typedef {{ units: number, withoutCode: number, realms: Record<string, Counts & { units: number }>, languages: Record<string, Record<string, Counts>> }} Totals */
/** @typedef {{ page: string, id: string, title: string, realm: Realm, gap: Gap }} PlacedGap */

export class CoverageTable {
  static realmNames = { tac: 'Tac', yon: 'Yon' }
  static empty = '-'

  /**
   * @param {UnitCoverage[]} coverages
   * @param {Record<Realm, string[]>} realmLanguages
   * @returns {Totals}
   */
  static totals(coverages, realmLanguages) {
    return {
      units: coverages.length,
      withoutCode: coverages.filter(coverage => coverage.realms.length === 0).length,
      realms: {
        tac: CoverageTable.realmTotals(coverages, 'tac'),
        yon: CoverageTable.realmTotals(coverages, 'yon'),
      },
      languages: {
        tac: CoverageTable.languageTotals(coverages, 'tac', realmLanguages.tac),
        yon: CoverageTable.languageTotals(coverages, 'yon', realmLanguages.yon),
      },
    }
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @returns {PlacedGap[]}
   */
  static unexplained(coverages) {
    return CoverageTable.placedGaps(coverages).filter(placed => CoverageTable.isUnexplained(placed.gap))
  }

  /**
   * @param {Gap} gap
   * @returns {boolean}
   */
  static isUnexplained(gap) {
    if (gap.kind === CoverageAudit.kinds.missing) return true
    return gap.kind === CoverageAudit.kinds.unsupported && !gap.stated
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @returns {PlacedGap[]}
   */
  static placedGaps(coverages) {
    return coverages.flatMap(coverage => coverage.realms.flatMap(realm => realm.gaps.map(gap => ({ ...coverage, realm: realm.realm, gap }))))
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @param {Realm} realm
   * @returns {Counts & { units: number }}
   */
  static realmTotals(coverages, realm) {
    const realms = CoverageTable.realmsNamed(coverages, realm)
    const present = realms.reduce((sum, coverage) => sum + coverage.present.length, 0)
    return { units: realms.length, ...CoverageTable.gapCounts(realms.flatMap(coverage => coverage.gaps), present) }
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @param {Realm} realm
   * @param {string[]} languages
   * @returns {Record<string, Counts>}
   */
  static languageTotals(coverages, realm, languages) {
    const realms = CoverageTable.realmsNamed(coverages, realm)
    return Object.fromEntries(languages.map(language => [language, CoverageTable.languageCounts(realms, language)]))
  }

  /**
   * @param {RealmCoverage[]} realms
   * @param {string} language
   * @returns {Counts}
   */
  static languageCounts(realms, language) {
    const present = realms.filter(coverage => coverage.present.includes(language)).length
    const gaps = realms.flatMap(coverage => coverage.gaps).filter(gap => gap.language === language)
    return CoverageTable.gapCounts(gaps, present)
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @param {Realm} realm
   * @returns {RealmCoverage[]}
   */
  static realmsNamed(coverages, realm) {
    return coverages.flatMap(coverage => coverage.realms).filter(coverage => coverage.realm === realm)
  }

  /**
   * @param {Gap[]} gaps
   * @param {number} present
   * @returns {Counts}
   */
  static gapCounts(gaps, present) {
    const unsupported = gaps.filter(gap => gap.kind === CoverageAudit.kinds.unsupported)
    return {
      present,
      missing: gaps.filter(gap => gap.kind === CoverageAudit.kinds.missing).length,
      unsupported: unsupported.length,
      unstated: unsupported.filter(gap => !gap.stated).length,
      notApplicable: gaps.filter(gap => gap.kind === CoverageAudit.kinds.notApplicable).length,
    }
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @param {Record<Realm, string[]>} realmLanguages
   * @returns {string}
   */
  static markdown(coverages, realmLanguages) {
    const totals = CoverageTable.totals(coverages, realmLanguages)
    return [
      '## Totals by realm',
      '',
      ...CoverageTable.realmRows(totals),
      '',
      `${totals.withoutCode} of ${totals.units} sections and features show no service-language code (views, configuration, commands or output), so no language is missing there.`,
      '',
      '## Totals by language',
      '',
      ...CoverageTable.languageRows(totals),
      '',
      '## Gaps by section',
      '',
      ...CoverageTable.gapRows(coverages),
      '',
    ].join('\n')
  }

  /**
   * @param {Totals} totals
   * @returns {string[]}
   */
  static realmRows(totals) {
    const rows = Object.entries(totals.realms).map(([realm, counts]) => CoverageTable.row([
      CoverageTable.realmName(realm), String(counts.units), ...CoverageTable.countCells(counts),
    ]))
    return [CoverageTable.row(['Realm', 'Sections', 'Tabs shown', 'Missing example', 'Not supported yet (not stated)', 'Not applicable']), CoverageTable.row(Array(6).fill('---')), ...rows]
  }

  /**
   * @param {Totals} totals
   * @returns {string[]}
   */
  static languageRows(totals) {
    const rows = Object.entries(totals.languages).flatMap(([realm, languages]) => Object.entries(languages).map(([language, counts]) => CoverageTable.row([
      CoverageTable.realmName(realm), CoverageAudit.languageName(language), ...CoverageTable.countCells(counts),
    ])))
    return [CoverageTable.row(['Realm', 'Language', 'Tabs shown', 'Missing example', 'Not supported yet (not stated)', 'Not applicable']), CoverageTable.row(Array(6).fill('---')), ...rows]
  }

  /**
   * @param {Counts} counts
   * @returns {string[]}
   */
  static countCells(counts) {
    return [String(counts.present), String(counts.missing), `${counts.unsupported} (${counts.unstated})`, String(counts.notApplicable)]
  }

  /**
   * @param {UnitCoverage[]} coverages
   * @returns {string[]}
   */
  static gapRows(coverages) {
    const header = ['Page', 'Topic or feature', 'Section', 'Realm', 'Shown', 'Missing example', 'Not supported yet', 'Not applicable']
    const rows = coverages.flatMap(coverage => coverage.realms.filter(realm => realm.gaps.length > 0).map(realm => CoverageTable.gapRow(coverage, realm)))
    return [CoverageTable.row(header), CoverageTable.row(Array(header.length).fill('---')), ...rows]
  }

  /**
   * @param {UnitCoverage} coverage
   * @param {RealmCoverage} realm
   * @returns {string}
   */
  static gapRow(coverage, realm) {
    return CoverageTable.row([
      coverage.page,
      coverage.id,
      coverage.title,
      CoverageTable.realmName(realm.realm),
      CoverageTable.names(realm.present),
      CoverageTable.names(CoverageTable.languagesOfKind(realm.gaps, CoverageAudit.kinds.missing)),
      CoverageTable.refusals(realm.gaps),
      CoverageTable.names(CoverageTable.languagesOfKind(realm.gaps, CoverageAudit.kinds.notApplicable)),
    ])
  }

  /**
   * @param {Gap[]} gaps
   * @param {string} kind
   * @returns {string[]}
   */
  static languagesOfKind(gaps, kind) {
    return gaps.filter(gap => gap.kind === kind).map(gap => gap.language)
  }

  /**
   * @param {Gap[]} gaps
   * @returns {string}
   */
  static refusals(gaps) {
    const refused = gaps.filter(gap => gap.kind === CoverageAudit.kinds.unsupported)
    if (refused.length === 0) return CoverageTable.empty
    return refused.map(gap => `${CoverageAudit.languageName(gap.language)} (${gap.code}, ${gap.stated ? 'stated' : 'not stated'})`).join(', ')
  }

  /**
   * @param {string[]} languages
   * @returns {string}
   */
  static names(languages) {
    if (languages.length === 0) return CoverageTable.empty
    return languages.map(language => CoverageAudit.languageName(language)).join(', ')
  }

  /**
   * @param {string} realm
   * @returns {string}
   */
  static realmName(realm) {
    return CoverageTable.realmNames[/** @type {Realm} */ (realm)]
  }

  /**
   * @param {string[]} cells
   * @returns {string}
   */
  static row(cells) {
    return `| ${cells.map(cell => cell.replaceAll('|', '\\|')).join(' | ')} |`
  }
}
