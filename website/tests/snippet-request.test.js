import { describe, expect, test } from 'bun:test'
import { SnippetRequest } from '../scripts/snippet-request.mjs'

describe('snippet requests follow the published schema', () => {
  test('strict records do not receive unrelated seed fields', () => {
    const schema = { $ref: '#/models/Message' }
    const models = { Message: { type: 'object', properties: { name: { type: 'string' } }, additionalProperties: false } }
    expect(SnippetRequest.body(schema, models)).toEqual({ name: 'snippet' })
  })

  test('nested records and lists use their own model fields', () => {
    const models = { Item: { type: 'object', properties: { sku: { type: 'string' }, level: { type: 'integer' } } } }
    const schema = { type: 'object', properties: { items: { type: 'array', items: { $ref: '#/models/Item' } } } }
    expect(SnippetRequest.body(schema, models)).toEqual({ items: [{ sku: 'anvil', level: 4 }] })
  })

  test('untyped bodies retain the complete probe payload', () => {
    expect(SnippetRequest.body({}, {})).toEqual({ name: 'snippet', sku: 'anvil', level: 4 })
  })

  test('recursive models stop at null or an empty list', () => {
    const node = { type: 'object', properties: { name: { type: 'string' }, next: { anyOf: [{ $ref: '#/models/Node' }, { type: 'null' }] } } }
    expect(SnippetRequest.body({ $ref: '#/models/Node' }, { Node: node })).toEqual({ name: 'snippet', next: null })
    const tree = { type: 'object', properties: { children: { type: 'array', items: { $ref: '#/models/Tree' } } } }
    expect(SnippetRequest.body({ $ref: '#/models/Tree' }, { Tree: tree })).toEqual({ children: [] })
  })

  test('missing references and endless records fail rather than guess', () => {
    expect(() => SnippetRequest.body({ $ref: '#/models/Missing' }, {})).toThrow('Missing snippet model')
    const cycle = { type: 'object', properties: { next: { $ref: '#/models/Cycle' } } }
    expect(() => SnippetRequest.body({ $ref: '#/models/Cycle' }, { Cycle: cycle })).toThrow('contains itself')
  })

  test('required query and header values are filled and optional ones are left out', () => {
    const fields = { limit: { type: 'integer' }, tags: { type: 'array', items: { type: 'string' } }, 'page?': { type: 'integer' } }
    expect(SnippetRequest.textValues(fields, {})).toEqual([['limit', '4'], ['tags', 'snippet']])
    expect(SnippetRequest.textValues(undefined, {})).toEqual([])
  })
})
