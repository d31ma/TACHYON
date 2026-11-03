import { describe, expect, test } from 'bun:test'
import { SnippetScope } from '../scripts/snippet-scope.mjs'

describe('verify-snippets picks the suite that the filters name', () => {
  test('a feature filter runs only the features suite', () => {
    expect(SnippetScope.choose({ feature: 'iteration' })).toBe('features')
  })

  test('a section filter runs only the guides suite', () => {
    expect(SnippetScope.choose({ section: 'Security' })).toBe('guides')
  })

  test('no filter runs every suite', () => {
    expect(SnippetScope.choose({})).toBe('all')
  })

  test('a written suite wins over the suite that a filter implies', () => {
    expect(SnippetScope.choose({ suite: 'tsc', feature: 'iteration' })).toBe('tsc')
  })

  test('an unknown suite is refused', () => {
    expect(() => SnippetScope.choose({ suite: 'everything' })).toThrow('Use --suite=all|features')
  })
})
