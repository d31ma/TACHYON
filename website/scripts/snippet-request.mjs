import assert from 'node:assert/strict'

export class SnippetRequest {
  static probe = { name: 'snippet', sku: 'anvil', level: 4 }
  static maxDepth = 32
  static text = 'snippet'
  static number = 4

  /**
   * @param {Record<string, any> | undefined} schema
   * @param {Record<string, any>} models
   * @param {any} seed
   * @param {number} depth
   * @param {string[]} expanding
   * @returns {any}
   */
  static body(schema, models, seed = SnippetRequest.probe, depth = 0, expanding = []) {
    assert.ok(depth < SnippetRequest.maxDepth, `Snippet body schema exceeds ${SnippetRequest.maxDepth} levels`)
    if (!schema || Object.keys(schema).length === 0) return seed
    if (typeof schema.$ref === 'string') {
      const name = SnippetRequest.modelName(schema, models)
      assert.ok(!expanding.includes(name), `Snippet model ${name} contains itself with no way to stop, such as null or an empty list`)
      return SnippetRequest.body(models[name], models, seed, depth + 1, [...expanding, name])
    }
    if (Array.isArray(schema.anyOf)) {
      const variant = schema.anyOf.find(value => value.type !== 'null')
      const allowsNull = schema.anyOf.some(value => value.type === 'null')
      if (!variant || (allowsNull && SnippetRequest.repeats(variant, models, expanding))) return null
      return SnippetRequest.body(variant, models, seed, depth + 1, expanding)
    }
    if (schema.type === 'object') {
      return Object.fromEntries(Object.entries(schema.properties ?? {}).map(([name, field]) => [
        name, SnippetRequest.body(field, models, seed?.[name] ?? SnippetRequest.probe, depth + 1, expanding),
      ]))
    }
    if (schema.type === 'array') {
      if (SnippetRequest.repeats(schema.items, models, expanding)) return []
      return [SnippetRequest.body(schema.items, models, seed?.[0] ?? SnippetRequest.probe, depth + 1, expanding)]
    }
    if (schema.type === 'string') return typeof seed === 'string' ? seed : SnippetRequest.text
    if (schema.type === 'integer' || schema.type === 'number') return typeof seed === 'number' ? seed : SnippetRequest.number
    if (schema.type === 'boolean') return typeof seed === 'boolean' ? seed : true
    if (schema.type === 'null') return null
    throw new Error(`Unsupported snippet body schema: ${JSON.stringify(schema)}`)
  }

  /**
   * @param {Record<string, any> | undefined} fields
   * @param {Record<string, any>} models
   * @returns {[string, string][]}
   */
  static textValues(fields, models) {
    /** @type {[string, string][]} */
    const values = []
    for (const [name, schema] of Object.entries(fields ?? {})) {
      if (name.endsWith('?')) continue
      const value = SnippetRequest.body(schema, models, undefined)
      for (const item of Array.isArray(value) ? value : [value]) values.push([name, String(item)])
    }
    return values
  }

  /**
   * @param {Record<string, any>} schema
   * @param {Record<string, any>} models
   */
  static modelName(schema, models) {
    assert.ok(schema.$ref.startsWith('#/models/'), 'Snippet schemas use local model references')
    const name = schema.$ref.slice('#/models/'.length)
    assert.ok(Object.hasOwn(models, name), `Missing snippet model: ${name}`)
    return name
  }

  /**
   * @param {Record<string, any> | undefined} schema
   * @param {Record<string, any>} models
   * @param {string[]} expanding
   */
  static repeats(schema, models, expanding) {
    return typeof schema?.$ref === 'string' && expanding.includes(SnippetRequest.modelName(schema, models))
  }
}
