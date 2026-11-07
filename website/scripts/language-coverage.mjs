import { CoverageAudit } from './coverage-audit.mjs'
import { CoverageTable } from './coverage-table.mjs'
import { SnippetUnits } from './snippet-units.mjs'

class LanguageCoverage {
  static jsonFlag = '--json'
  static strictFlag = '--strict'
  static jsonIndent = 2

  static run() {
    const audit = CoverageAudit.load()
    const coverages = audit.audit(SnippetUnits.collect())
    const output = LanguageCoverage.render(audit, coverages)
    console.log(output)
    LanguageCoverage.enforce(coverages)
  }

  /**
   * @param {CoverageAudit} audit
   * @param {import('./coverage-audit.mjs').UnitCoverage[]} coverages
   * @returns {string}
   */
  static render(audit, coverages) {
    if (!process.argv.includes(LanguageCoverage.jsonFlag)) return CoverageTable.markdown(coverages, audit.realmLanguages)
    const report = {
      languages: audit.realmLanguages,
      checkedByRuleTable: audit.reach.realmLanguages,
      totals: CoverageTable.totals(coverages, audit.realmLanguages),
      sections: coverages,
    }
    return JSON.stringify(report, null, LanguageCoverage.jsonIndent)
  }

  /** @param {import('./coverage-audit.mjs').UnitCoverage[]} coverages */
  static enforce(coverages) {
    if (!process.argv.includes(LanguageCoverage.strictFlag)) return
    const unexplained = CoverageTable.unexplained(coverages)
    if (unexplained.length === 0) return
    console.error(`${unexplained.length} language gaps have no example and no stated reason. Add the example, or say in the section which build error refuses it.`)
    process.exitCode = 1
  }
}

LanguageCoverage.run()
