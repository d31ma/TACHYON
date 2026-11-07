import { describe, expect, test } from 'bun:test'
import { SnippetShard } from '../scripts/snippet-shard.mjs'

describe('verify-snippets splits its steps into shards', () => {
  test('no shard option runs every step', () => {
    expect(SnippetShard.parse(undefined).select(['a', 'b', 'c'])).toEqual(['a', 'b', 'c'])
  })

  test('a shard takes every count-th step, starting at its index', () => {
    expect(SnippetShard.parse('2/3').select(['a', 'b', 'c', 'd', 'e', 'f', 'g'])).toEqual(['b', 'e'])
  })

  test('a shard keeps counting across separate step lists', () => {
    const shard = SnippetShard.parse('1/2')
    expect([shard.select(['a', 'b', 'c']), shard.select(['d', 'e'])]).toEqual([['a', 'c'], ['e']])
  })

  test('the shards of one count together take every step exactly once', () => {
    const steps = ['a', 'b', 'c', 'd', 'e', 'f', 'g']
    const taken = [
      ...SnippetShard.parse('1/3').select(steps),
      ...SnippetShard.parse('2/3').select(steps),
      ...SnippetShard.parse('3/3').select(steps),
    ]
    expect(taken.sort()).toEqual(steps)
  })

  test('only the first shard runs the checks that are not split', () => {
    expect([SnippetShard.parse('1/3').ownsSingleChecks(), SnippetShard.parse('2/3').ownsSingleChecks()]).toEqual([true, false])
  })

  test('a shard describes itself for the run header', () => {
    expect(SnippetShard.parse('3/4').describe()).toBe('3 of 4')
  })

  test('a malformed shard is refused', () => {
    expect(() => SnippetShard.parse('2-3')).toThrow('Use --shard=<index>/<count>')
  })

  test('a shard index above the count is refused', () => {
    expect(() => SnippetShard.parse('4/3')).toThrow('the index 4 is above the count 3')
  })

  test('a zero shard index is refused', () => {
    expect(() => SnippetShard.parse('0/3')).toThrow('Use --shard=<index>/<count>')
  })
})
