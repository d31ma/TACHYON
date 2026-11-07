import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { spawnSync } from 'node:child_process'
import { brotliCompressSync, gzipSync } from 'node:zlib'
import { chromium } from 'playwright'
import { TachyonRuby } from '../runtimes/ruby-web/tachyon-ruby.js'

class RubyRuntimeTest {
  static folder = new URL('../runtimes/ruby-web/', import.meta.url)
  static dispatcher = `module TachyonWeb
    def self.invoke(text)
      JSON.generate(eval(JSON.parse(text)['source']))
    end
  end`
  static cases = [
    ['JSON values', `JSON.parse(JSON.generate({'name'=>'café 😀','items'=>[nil,true,false,-42,1.25],'escaped'=>"\\n\\t\\\\\\\""}))`],
    ['surrogate pair', `JSON.parse('"\\ud83d\\ude00"')`],
    ['symbol keys', `JSON.parse('{"name":"Ruby"}', symbolize_names: true).keys.map(&:to_s)`],
    ['large integers', `[9223372036854775807, 9223372036854775808, -9223372036854775809].map { |value| JSON.parse(JSON.generate(value)).to_s }`],
    ['range steps', `[(1..5).step(2).to_a, (1...5).step(2).to_a]`],
    ['JSON malformed surrogate', `begin; JSON.parse('"\\ud800"'); rescue JSON::ParserError; 'refused'; end`],
    ['zero step', `begin; (1..3).step(0).to_a; rescue ArgumentError; 'refused'; end`],
    ['negative step', `(5..1).step(-2).to_a`],
    ['JSON malformed number', `begin; JSON.parse('01'); rescue JSON::ParserError; 'refused'; end`],
    ['JSON nested limit', `begin; JSON.parse('[' * 101 + '0' + ']' * 101); rescue JSON::NestingError; 'refused'; end`],
    ['JSON circular value', `value = []; value << value; begin; JSON.generate(value); rescue JSON::NestingError; 'refused'; end`],
    ['JSON nonfinite', `begin; JSON.generate(Float::INFINITY); rescue JSON::GeneratorError; 'refused'; end`],
    ['collections', `[{a: 1, b: 2}.transform_values { |value| value * 2 }, [1,2,2].uniq, (1..100).sum]`],
  ]

  static request(source) {
    return JSON.stringify({ source })
  }

  static runtimePath() {
    const path = process.argv[2]
    assert.ok(path, 'usage: node scripts/ruby-runtime-test.mjs <built tachyon-ruby.wasm>')
    return path
  }

  static async artifact() {
    const bytes = await readFile(this.runtimePath())
    const checksum = createHash('sha256').update(bytes).digest('hex')
    assert.ok(bytes.length < 4 * 1024 * 1024, 'runtime must fit the service-worker precache budget')
    const module = await WebAssembly.compile(bytes)
    const exports = WebAssembly.Module.exports(module).map(value => value.name)
    for (const name of ['memory', '_initialize', 'tac_boot', 'tac_invoke', 'tac_alloc', 'tac_free']) assert.ok(exports.includes(name), name)
    const imports = WebAssembly.Module.imports(module)
    assert.equal(imports.length, 9)
    assert.deepEqual(imports.filter(value => value.module === 'tachyon'), [{ module: 'tachyon', name: 'tac_host_emit', kind: 'function' }])
    console.log(JSON.stringify({ bytes: bytes.length, gzip: gzipSync(bytes).length, brotli: brotliCompressSync(bytes).length, checksum, imports }))
    return { bytes, module }
  }

  static nativeAnswer(source) {
    const result = spawnSync('ruby', ['-EUTF-8', '-rjson', '-e', 'STDOUT.write(JSON.generate(eval(STDIN.read)))'], {
      input: source, encoding: 'utf8', timeout: 10_000,
    })
    assert.equal(result.status, 0, result.stderr)
    return JSON.parse(result.stdout)
  }

  static allocationFailures() {
    const runtime = new TachyonRuby(() => {}, () => {})
    runtime.exports = { tac_alloc: () => 0 }
    assert.throws(() => runtime.invoke('{}'), /could not allocate/)
    let freed = false
    runtime.exports = {
      memory: { buffer: Uint8Array.from([1, 2, 3]).buffer },
      tac_free: () => { freed = true },
    }
    assert.throws(() => runtime.takeAnswer(1), /unterminated answer/)
    assert.equal(freed, true)
    console.log('PASS allocation failure and malformed ABI answer cleanup')
  }

  static consoleBoundaries() {
    const lines = []
    const runtime = new TachyonRuby((line, descriptor) => lines.push([line, descriptor]), () => {})
    runtime.exports = { memory: new WebAssembly.Memory({ initial: 2 }) }
    const write = (descriptor, bytes) => {
      runtime.memory().set(bytes, 32)
      runtime.view().setUint32(0, 32, true)
      runtime.view().setUint32(4, bytes.length, true)
      assert.equal(runtime.writeOutput(descriptor, 0, 1, 8), 0)
      assert.equal(runtime.view().getUint32(8, true), bytes.length)
    }
    write(1, Uint8Array.from([0xf0, 0x9f]))
    write(2, new TextEncoder().encode('error\n'))
    write(1, Uint8Array.from([0x98, 0x80, 0x0a]))
    assert.deepEqual(lines, [['error', 2], ['😀', 1]])
    write(1, new TextEncoder().encode('x'.repeat(TachyonRuby.OUTPUT_LINE_LIMIT + 10)))
    write(1, new TextEncoder().encode('y'.repeat(TachyonRuby.OUTPUT_LINE_LIMIT)))
    assert.equal(lines.length, 3)
    assert.equal(lines[2][0], 'x'.repeat(TachyonRuby.OUTPUT_LINE_LIMIT) + TachyonRuby.OUTPUT_TRUNCATED)
    assert.equal(runtime.outputStream(1).pending.length, 0)
    write(1, new TextEncoder().encode('\nrecovered\n'))
    assert.deepEqual(lines[3], ['recovered', 1])
    assert.equal(runtime.writeOutput(3, 0, 0, 8), TachyonRuby.BAD_DESCRIPTOR)
    console.log('PASS bounded console lines, split UTF-8, separate descriptors and recovery')
  }

  static async protocol(module) {
    const events = []
    const output = []
    const runtime = await TachyonRuby.start(module, line => output.push(line), text => events.push(JSON.parse(text)))
    assert.deepEqual(JSON.parse(runtime.invoke(this.request('1'))), { error: 'not booted' })
    assert.equal(runtime.boot(this.dispatcher), null)
    for (const [name, source] of this.cases) {
      const answer = JSON.parse(runtime.invoke(this.request(source)))
      assert.deepEqual(answer, this.nativeAnswer(source), name)
      console.log(`PASS CRuby/mruby: ${name}`)
    }
    assert.equal(JSON.parse(runtime.invoke(this.request(`TachyonWeb.host_emit(JSON.generate({event:'order.created', value:{id:7}, retain:true})); nil`))), null)
    assert.deepEqual(events, [{ event: 'order.created', value: { id: 7 }, retain: true }])
    assert.equal(JSON.parse(runtime.invoke(this.request(`puts 'Ruby output'; 42`))), 42)
    assert.deepEqual(output, ['Ruby output'])
    assert.equal(JSON.parse(runtime.invoke(this.request(`STDOUT.write('x' * 65546); puts 'tail'; puts 'next'; 42`))), 42)
    assert.equal(output[1], 'x'.repeat(TachyonRuby.OUTPUT_LINE_LIMIT) + TachyonRuby.OUTPUT_TRUNCATED)
    assert.equal(output[2], 'next')
    assert.match(JSON.parse(runtime.boot('class')).error, /SyntaxError/)
    assert.match(JSON.parse(runtime.invoke(this.request(`raise 'expected failure'`))).error, /expected failure/)
    assert.match(JSON.parse(runtime.invoke(this.request(`JSON.parse('[1, x]')`))).error, /unexpected token at offset 4$/)
    assert.match(JSON.parse(runtime.invoke(this.request(`JSON.parse('{"abc": 1,}')`))).error, /expected object key at offset 10$/)
    assert.equal(JSON.parse(runtime.invoke(this.request('6 * 7'))), 42)
    assert.deepEqual(JSON.parse(runtime.invoke(this.request(`[defined?(File), defined?(IO), defined?(Socket), defined?(ENV)]`))), [null, null, null, null])
    assert.match(JSON.parse(runtime.invoke(this.request(`require 'socket'`))).error, /LoadError/)
    assert.match(JSON.parse(runtime.invoke(this.request(`sleep 1`))).error, /synchronous/)
    console.log(`PASS synchronous ABI, events, recovery and capability boundary; memory ${runtime.memoryBytes()} bytes`)
  }

  static async hostFailures(module) {
    const failing = () => { throw new Error('host failed') }
    const runtime = await TachyonRuby.start(module, failing, failing)
    assert.equal(runtime.boot(this.dispatcher), null)
    for (let round = 0; round < 300; round += 1) {
      assert.throws(() => runtime.invoke(this.request(`[1].map { TachyonWeb.host_emit('x') }; 1`)), /host failed/)
    }
    assert.throws(() => runtime.invoke(this.request(`puts 'line'; 1`)), /host failed/)
    const rescued = runtime.invoke(this.request(`begin; raise 'inner'; rescue => error; error.message; end`))
    assert.equal(JSON.parse(rescued), 'inner')
    console.log('PASS failing host callbacks do not corrupt the Ruby state')
  }

  static async browser(bytes) {
    const loader = await readFile(new URL('tachyon-ruby.js', this.folder))
    const script = `import { TachyonRuby } from '/runtime.js';
      const events = [];
      const ruby = await TachyonRuby.start('/runtime.wasm', () => {}, text => events.push(JSON.parse(text)));
      const boot = ruby.boot(${JSON.stringify(this.dispatcher)});
      const answer = JSON.parse(ruby.invoke(${JSON.stringify(this.request(`TachyonWeb.host_emit(JSON.generate({event:'ready', value:42})); 42`))}));
      document.body.dataset.result = JSON.stringify({boot, answer, events});`
    const server = createServer((request, response) => {
      response.setHeader('Content-Security-Policy', "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'")
      const resources = {
        '/': ['text/html', '<script type="module" src="/test.js"></script><body></body>'],
        '/test.js': ['text/javascript', script],
        '/runtime.js': ['text/javascript', loader],
        '/runtime.wasm': ['application/wasm', bytes],
      }
      const resource = resources[request.url]
      if (!resource) { response.writeHead(404).end(); return }
      response.setHeader('Content-Type', resource[0])
      response.end(resource[1])
    })
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
    let browser
    try {
      browser = await chromium.launch({ headless: true })
      const page = await browser.newPage()
      const errors = []
      page.on('pageerror', error => errors.push(error.message))
      await page.goto(`http://127.0.0.1:${server.address().port}`)
      await page.waitForFunction(() => document.body.dataset.result)
      assert.deepEqual(await page.evaluate(() => JSON.parse(document.body.dataset.result)), {
        boot: null, answer: 42, events: [{ event: 'ready', value: 42 }],
      })
      assert.deepEqual(errors, [])
      console.log('PASS browser strict CSP: no unsafe-eval, inline script, blob, CDN or installed Ruby toolchain')
    } finally {
      await browser?.close()
      await new Promise(resolve => server.close(resolve))
    }
  }

  static async main() {
    this.allocationFailures()
    this.consoleBoundaries()
    const { bytes, module } = await this.artifact()
    await this.protocol(module)
    await this.hostFailures(module)
    await this.browser(bytes)
  }
}

await RubyRuntimeTest.main()
