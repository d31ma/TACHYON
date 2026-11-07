import assert from 'node:assert/strict'

/** @typedef {{suite?:string,feature?:string,section?:string}} ScopeOptions */

export class SnippetScope {
  static suites = ['all', 'features', 'languages', 'guides', 'configuration', 'layout', 'annotations', 'gofmt', 'ruby', 'python', 'tsc']

  /**
   * @param {ScopeOptions} options
   * @returns {string}
   */
  static choose(options) {
    const suite = options.suite ?? SnippetScope.implied(options)
    assert.ok(SnippetScope.suites.includes(suite), `Use --suite=${SnippetScope.suites.join('|')}`)
    return suite
  }

  /**
   * @param {ScopeOptions} options
   * @returns {string}
   */
  static implied(options) {
    if (options.feature) return 'features'
    if (options.section) return 'guides'
    return 'all'
  }
}
