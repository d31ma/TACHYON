#!/usr/bin/env node
import assert from 'node:assert/strict'
import { AsyncLocalStorage } from 'node:async_hooks'
import { createHash } from 'node:crypto'
import { spawn } from 'node:child_process'
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs'
import net from 'node:net'
import { cpus, loadavg, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { withServer } from '../../scripts/release/server-probe.mjs'
import { checkConfigurationBrowser, configurationTestFiles } from './verify-configuration-binding.mjs'
import { SiteContent } from './site-content.mjs'
import { SharedModuleProbe } from './shared-module-probe.mjs'
import { SnippetCache } from './snippet-cache.mjs'
import { SnippetRequest } from './snippet-request.mjs'
import { SnippetScope } from './snippet-scope.mjs'
import { SocketProbe } from './socket-probe.mjs'
import { ToolchainFingerprint } from './toolchain-fingerprint.mjs'
import { TypeCheckProjects } from './type-check-projects.mjs'

/** @typedef {{name:string,code:string}} Snippet */
/** @typedef {{id:string,files:Snippet[],verify?:string,target?:string}} Feature */
/** @typedef {{heading:string,axis?:string,code?:string,target?:string,files?:Snippet[]}} GuideSection */
/** @typedef {{status:number,type:string,value:any}} ExpectedError */

class Limits {
  static outputBytes = 1024 * 1024
  static responseBytes = 65536
  static removeStagingAfterMilliseconds = 7 * 24 * 60 * 60 * 1000
}

class Settings {
  static repo = fileURLToPath(new URL('../..', import.meta.url))
  static binary = findCheckedBinary(Settings.repo)
  static bun = process.env.TAC_BUN ?? process.env.YON_JAVASCRIPT_RUNTIME ?? 'bun'
  static {
    prepareToolEnvironment(Settings.repo, Settings.bun)
  }
  static scope = readCheckedScope()
  static onlyLanguage = readOption('--language=')
  static onlySection = readOption('--section=')
  static onlyFeature = readOption('--feature=')
  static scratchRoot = prepareScratchRoot()
  static cache = new SnippetCache(path.join(Settings.scratchRoot, SnippetCache.folderName), !process.argv.includes('--no-cache'), '')
  static concurrency = chooseConcurrency()
  static requireToolchains = process.env.TAC_SNIPPETS_REQUIRE_TOOLCHAINS === '1'
  static identityToken = 'ada-token'
  static loadFailureLimit = Number(process.env.TAC_SNIPPETS_READINESS_BUDGET ?? 2)
}

class SiteData {
  /** @type {Feature[]} */
  static features = SiteContent.data('features.json').features
  /** @type {{tac:{files:Snippet[]},yon:{entries:{id:string,files:Snippet[]}[]}}} */
  static languages = SiteContent.data('languages.json')
  /** @type {{topics:Record<string,{sections?:GuideSection[]}>}} */
  static docs = SiteContent.data('docs.json')
}

class Progress {
  static counts = { bundle: 0, browser: 0, http: 0, unit: 0, syntax: 0, types: 0, data: 0, skipped: 0, failed: 0 }
  static started = Date.now()
  static previousStepEnd = Progress.started
  /** @type {{label:string,kind:string,ms:number}[]} */
  static timings = []
  /** @type {AsyncLocalStorage<{lines:[string,string][], start:number}>} */
  static currentStep = new AsyncLocalStorage()
}

class Patterns {
  static tutorials = new Set(['first-app'])
  static excerptManifest = JSON.stringify({ tac: { visibility: { check: false } } })
  static roleFile = /\/(controllers|services|stores|clients|helpers|setups|converters|entities)\/[^/]+\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|swift|dart|go|rb|lua)$/
  static controllerPath = /^server\/controllers\/[A-Z]\w*Controller\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|dart|go|rb|lua)$/
  static identityProviderPath = /^server\/controllers\/[A-Z]\w*IdentityProvider\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|dart|go|rb|lua)$/
  static classRoute = /^\s*(?:\/\/ @|-- @|@|#\[|\[<|\[)(?:RequestRoute|request_route)\b(?:\(\s*["']([^"']*)["']\s*\))?/m
  static socketRoute = /^\s*(?:\/\/ @|-- @|@|#\[|\[<|\[)(?:SocketRoute|socket_route)\b(?:\(\s*["']([^"']*)["']\s*\))?/m
  static methodRoute = /^\s*(?:\/\/ @|-- @|@|#\[|\[<|\[)(Get|Post|Put|Patch|Delete|Stream|get|post|put|patch|delete|stream)(?:Route|_route)\b(?:\(\s*["']([^"']*)["']\s*\))?/gm
  static pathVariable = /\{[a-z0-9_]+\}/g
  static pagePath = /^client\/pages\/.*tac\.html$/
  static componentViewPath = /^client\/components\/.*\/tac\.html$/
  static servicePath = /^client\/services\/[A-Z]\w*Service\.(js|ts|dart|swift|kt|java|cs|fs|rs|go|rb)$/
  static testUnitPath = /^(server|client)\/tests\/[A-Z]\w*Test\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|swift|dart|go|rb|lua)$/
  static lastSourceExtension = /\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|swift|dart|go|rb|lua)\b(?!.*\.(?:js|ts|py|php|java|groovy|kt|cs|fs|rs|swift|dart|go|rb|lua)\b)/
  static languageTag = /\[(js|ts|py|php|java|groovy|kt|cs|fs|rs|swift|dart|go|rb|lua)\]/
  static handlerService = /\son:[\w-]+="\s*([A-Za-z_$][\w$]*)\.[A-Za-z_$]/g
  static serviceExcerpt = /^@Service(?:\(\))?\s*\n/
  static loadFailure = /Server readiness timed out|exceeded \d+ms|deadline|aborted due to timeout/
  static sharedFile = /^server\/shared\/.+\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|dart|go|rb|lua)$/
  static middlewareFile = /^middleware\.(js|ts|py|php|java|groovy|kt|cs|fs|rs|dart|go|rb|lua)$/
  static typeScaffoldFiles = ['package.json', 'tachyon-env.d.ts', 'tachyon-behaviors.d.ts', 'client/jsconfig.json', 'server/jsconfig.json']
  static middlewareController = '@RequestRoute("/private")\n@Public\nclass PrivateController {\n  @GetRoute\n  getPrivate() {\n    return { secret: 42 }\n  }\n}\n'
}

class Languages {
  static sourceExtensions = new Set(['.js', '.ts', '.py', '.php', '.java', '.groovy', '.kt', '.cs', '.fs', '.rs', '.swift', '.dart', '.go', '.rb', '.lua'])
  /** @type {Record<string,string>} */
  static nativeTargets = { '.swift': 'macos', '.kt': 'android', '.cs': 'windows', '.rs': 'linux' }
  static tacSourceExtensions = new Set(['.js', '.ts', '.dart', '.go', '.rb', '.fs', '.java', ...Object.keys(Languages.nativeTargets)])
}

class Toolchains {
  /** @type {Record<string,string[]>} */
  static requiredTools = {
    '.js': [Settings.bun],
    '.ts': [Settings.bun],
    '.py': ['python3'],
    '.php': ['php'],
    '.java': ['javac', 'java'],
    '.groovy': ['groovyc', 'java'],
    '.kt': ['kotlinc', 'java'],
    '.cs': ['dotnet'],
    '.fs': ['dotnet'],
    '.rs': ['rustc'],
    '.dart': ['dart'],
    '.go': ['go'],
    '.rb': ['ruby'],
    '.lua': ['lua'],
  }
  /** @type {Map<string,Promise<boolean>>} */
  static checks = new Map()
  /** @type {Map<string, {count:number, reason:string}>} */
  static loadFailures = new Map()
}

class Staging {
  static activeDirectories = new Set()
}

/** @param {string} repo */
function findBuiltBinary(repo) {
  const targetDirectory = path.resolve(repo, process.env.CARGO_TARGET_DIR ?? 'target')
  const executableName = process.platform === 'win32' ? 'ty.exe' : 'ty'
  for (const mode of ['debug', 'release']) {
    const candidate = path.join(targetDirectory, mode, executableName)
    if (existsSync(candidate)) return candidate
  }
  return undefined
}

/** @param {string} repo */
function findCheckedBinary(repo) {
  const selected = process.env.TAC_BIN ?? findBuiltBinary(repo)
  assert.ok(selected, 'Build this checkout first or set TAC_BIN to its binary; installed/PATH ty binaries are never selected')
  const binary = path.resolve(selected)
  assert.ok(existsSync(binary), `TAC_BIN does not exist: ${binary}`)
  return binary
}

/**
 * @param {string} repo
 * @param {string} bun
 */
function prepareToolEnvironment(repo, bun) {
  const toolchainFile = readFileSync(path.join(repo, 'rust-toolchain.toml'), 'utf8')
  const pinnedToolchain = toolchainFile.match(/channel\s*=\s*"([^"]+)"/)?.[1]
  if (pinnedToolchain && !process.env.RUSTUP_TOOLCHAIN) process.env.RUSTUP_TOOLCHAIN = pinnedToolchain
  const searchPath = [
    path.join(repo, 'node_modules/.bin'),
    path.join(process.env.HOME ?? '', '.cargo/bin'),
  ]
  if (process.env.GOROOT) searchPath.push(path.join(process.env.GOROOT, 'bin'))
  searchPath.push(process.env.PATH ?? '')
  process.env.PATH = searchPath.join(path.delimiter)
  process.env.YON_JAVASCRIPT_RUNTIME = bun
  process.env.TAC_ALLOW_UNTESTED = '1'
}

/** @param {string} prefix */
function readOption(prefix) {
  const argument = process.argv.find(value => value.startsWith(prefix))
  return argument?.slice(prefix.length)
}

function readCheckedScope() {
  return SnippetScope.choose({ suite: readOption('--suite='), feature: readOption('--feature='), section: readOption('--section=') })
}

/** @param {string} name */
function isSelectedLanguage(name) {
  if (!Settings.onlyLanguage) return true
  return path.extname(name) === `.${Settings.onlyLanguage}`
}

function chooseScratchRoot() {
  return process.env.TAC_SNIPPETS_SCRATCH || path.join(tmpdir(), 'tachyon-snippets')
}

/** @param {string} scratchRoot */
function removeOldStagingDirectories(scratchRoot) {
  for (const entry of readdirSync(scratchRoot, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.name.startsWith('.')) continue
    const directory = path.join(scratchRoot, entry.name)
    try {
      const age = Date.now() - statSync(directory).mtimeMs
      if (age > Limits.removeStagingAfterMilliseconds) rmSync(directory, { recursive: true, force: true })
    } catch {}
  }
}

function prepareScratchRoot() {
  const scratchRoot = chooseScratchRoot()
  mkdirSync(scratchRoot, { recursive: true })
  process.env.TAC_CACHE_DIR ??= path.join(scratchRoot, '.runtime-cache')
  removeOldStagingDirectories(scratchRoot)
  return scratchRoot
}

function chooseConcurrency() {
  const configured = Number(process.env.TAC_SNIPPETS_CONCURRENCY)
  if (Number.isFinite(configured) && configured >= 1) return Math.floor(configured)
  const idleCores = cpus().length - loadavg()[0]
  return Math.min(4, Math.max(1, Math.floor(idleCores / 2)))
}

/**
 * @param {Snippet[]} files
 * @param {Snippet} handler
 */
function filesForHandlerLanguage(files, handler) {
  const extension = path.extname(handler.name)
  const sameLanguage = files.filter(file => path.extname(file.name) === extension)
  const gatewayFiles = gatewayFileNames(sameLanguage)
  const foreignGatewayFiles = gatewayFileNames(files.filter(file => path.extname(file.name) !== extension))
  const sameLanguageCode = sameLanguage.map(file => file.code).join('\n')
  return files.filter(file => {
    if (file.name.startsWith('client/')) return false
    if (foreignGatewayFiles.has(file.name) && !isNamedByOtherFiles(file, sameLanguage)) return false
    const fileExtension = path.extname(file.name)
    const realmImportPath = `/${file.name.split('/').slice(1).join('/')}`
    if (gatewayFiles.has(file.name) || sameLanguageCode.includes(file.name) || sameLanguageCode.includes(realmImportPath)) return true
    return !Languages.sourceExtensions.has(fileExtension) || fileExtension === extension
  })
}

/**
 * @param {Snippet} file
 * @param {Snippet[]} sameLanguage
 */
function isNamedByOtherFiles(file, sameLanguage) {
  const className = path.basename(file.name, path.extname(file.name))
  return sameLanguage.some(other => other.name !== file.name && other.code.includes(className))
}

/**
 * @param {string} code
 * @returns {{class?: string, method?: string, command?: string[]}[]}
 */
function gatewayTargets(code) {
  const targets = []
  for (const match of code.matchAll(/^\s*(?:\/\/ @|-- @|@|#\[|\[<|\[)[Gg]ateway\((\{.*\})\)\]?\s*$/gm)) {
    try {
      targets.push(JSON.parse(match[1]))
    } catch {
      continue
    }
  }
  return targets
}

/** @param {Snippet[]} files */
function gatewayFileNames(files) {
  const names = new Set()
  for (const file of files) {
    for (const target of gatewayTargets(file.code)) {
      if (typeof target.class === 'string') names.add(target.class)
      for (const entry of target.command ?? []) {
        if (entry.includes('/')) names.add(entry)
      }
    }
  }
  return names
}

/** @param {Snippet[]} files */
function gatewayTools(files) {
  const tools = new Set()
  for (const file of files) {
    for (const target of gatewayTargets(file.code)) {
      if (Array.isArray(target.command) && target.command.length > 0) tools.add(target.command[0])
      if (typeof target.class !== 'string') continue
      for (const tool of Toolchains.requiredTools[path.extname(target.class)] ?? []) tools.add(tool)
    }
  }
  return [...tools]
}

/** @param {Snippet[]} files */
function hasRoleFiles(files) {
  return files.some(file => Patterns.roleFile.test(file.name))
}

/** @param {Snippet} file */
function isPageFile(file) {
  return Patterns.pagePath.test(file.name)
}

/** @param {string} name */
function isService(name) {
  return Patterns.servicePath.test(name)
}

/** @param {string} name */
function isTestUnit(name) {
  return Patterns.testUnitPath.test(name)
}

/**
 * @param {'log'|'error'} stream
 * @param {string} text
 */
function emit(stream, text) {
  const step = Progress.currentStep.getStore()
  if (step) {
    step.lines.push([stream, text])
    return
  }
  if (stream === 'error') console.error(text)
  else console.log(text)
}

/**
 * @param {string} label
 * @param {string} kind
 */
function record(label, kind) {
  const now = Date.now()
  const step = Progress.currentStep.getStore()
  if (step) {
    Progress.timings.push({ label, kind, ms: now - step.start })
    step.start = now
    return
  }
  Progress.timings.push({ label, kind, ms: now - Progress.previousStepEnd })
  Progress.previousStepEnd = now
}

/**
 * @param {string} label
 * @param {'bundle'|'browser'|'http'|'unit'|'syntax'|'types'|'data'} kind
 */
function pass(label, kind) {
  Progress.counts[kind]++
  record(label, kind)
  emit('log', `  ok  ${label} [${kind}]`)
}

/**
 * @param {string} label
 * @param {string} reason
 */
function skip(label, reason) {
  Progress.counts.skipped++
  record(label, 'skipped')
  emit('log', `  --  ${label} [not executed: ${reason}]`)
}

/** @param {string} label */
function labelExtension(label) {
  const match = label.match(Patterns.lastSourceExtension) ?? label.match(Patterns.languageTag)
  return match ? `.${match[1]}` : ''
}

/** @param {unknown} error */
function describeError(error) {
  const text = String(error instanceof Error ? error.stack : error)
  const firstLines = text.slice(0, 8192).split('\n').slice(0, 12)
  return firstLines.join('\n')
}

/** @param {unknown} error */
function errorMessage(error) {
  return String(error instanceof Error ? error.message : error)
}

/**
 * @param {string} label
 * @param {unknown} error
 */
function fail(label, error) {
  Progress.counts.failed++
  record(label, 'failed')
  recordLoadFailure(labelExtension(label), error)
  emit('error', `FAIL ${label}\n${describeError(error)}`)
}

/** @param {(() => Promise<void>)[]} steps */
async function runInParallel(steps) {
  if (Settings.concurrency === 1 || steps.length < 2) {
    for (const step of steps) await step()
    return
  }
  const outputLines = steps.map(() => /** @type {[string,string][]} */ ([]))
  const finished = steps.map(() => false)
  let printedCount = 0
  const printFinishedStepsInOrder = () => {
    while (printedCount < steps.length && finished[printedCount]) {
      for (const [stream, text] of outputLines[printedCount]) {
        if (stream === 'error') console.error(text)
        else console.log(text)
      }
      printedCount += 1
    }
  }
  let nextIndex = 0
  const worker = async () => {
    while (nextIndex < steps.length) {
      const index = nextIndex++
      await Progress.currentStep.run({ lines: outputLines[index], start: Date.now() }, steps[index])
      finished[index] = true
      printFinishedStepsInOrder()
    }
  }
  const workers = []
  const workerCount = Math.min(Settings.concurrency, steps.length)
  for (let count = 0; count < workerCount; count += 1) workers.push(worker())
  await Promise.all(workers)
  Progress.previousStepEnd = Date.now()
}

/**
 * @param {string} command
 * @param {string[]} args
 * @returns {Promise<string>}
 */
function runCommand(command, args, { cwd = Settings.repo, timeout = 120_000 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, detached: process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'] })
    let stdout = ''
    let stderr = ''
    /** @type {Error|undefined} */
    let failure
    /** @type {ReturnType<typeof setTimeout>|undefined} */
    let killTimer
    /** @param {NodeJS.Signals} signal */
    const sendSignal = signal => {
      try {
        if (process.platform === 'win32') child.kill(signal)
        else if (child.pid) process.kill(-child.pid, signal)
      } catch {}
    }
    /** @param {string} message */
    const terminate = message => {
      if (failure) return
      failure = new Error(message)
      sendSignal('SIGTERM')
      killTimer = setTimeout(() => sendSignal('SIGKILL'), 2000)
    }
    const timeoutTimer = setTimeout(() => terminate(`${command}: exceeded ${timeout}ms`), timeout)
    child.stdout.on('data', chunk => {
      if (failure) return
      stdout += chunk
      if (Buffer.byteLength(stdout) > Limits.outputBytes) terminate(`${command}: stdout limit exceeded`)
    })
    child.stderr.on('data', chunk => {
      if (failure) return
      stderr += chunk
      if (Buffer.byteLength(stderr) > Limits.outputBytes) terminate(`${command}: stderr limit exceeded`)
    })
    child.once('error', error => { failure = error })
    child.once('close', code => {
      clearTimeout(timeoutTimer)
      clearTimeout(killTimer)
      if (failure) {
        reject(failure)
        return
      }
      if (code !== 0) {
        reject(new Error(`${command} exited ${code}:\n${stdout}${stderr}`))
        return
      }
      resolve(stdout)
    })
  })
}

/** @param {string} tool */
function versionFlag(tool) {
  if (tool === 'javac' || tool === 'kotlinc' || tool === 'kotlinc-js') return '-version'
  if (tool === 'go') return 'version'
  if (tool === 'lua' || tool === 'luac') return '-v'
  return '--version'
}

/** @param {string} tool */
function isToolAvailable(tool) {
  if (!Toolchains.checks.has(tool)) {
    const check = runCommand(tool, [versionFlag(tool)], { timeout: 15_000 }).then(() => true, () => false)
    Toolchains.checks.set(tool, check)
  }
  return Toolchains.checks.get(tool)
}

/**
 * @param {number} port
 * @returns {Promise<boolean>}
 */
function isPortListening(port, host = '127.0.0.1') {
  return new Promise(resolve => {
    const socket = net.connect({ port, host }, () => {
      socket.destroy()
      resolve(true)
    })
    socket.setTimeout(1000, () => {
      socket.destroy()
      resolve(false)
    })
    socket.on('error', () => resolve(false))
  })
}

/** @param {unknown} error */
function isLoadFailure(error) {
  return Patterns.loadFailure.test(errorMessage(error))
}

/**
 * @param {string} extension
 * @param {unknown} error
 */
function recordLoadFailure(extension, error) {
  if (!extension || !isLoadFailure(error)) return
  const firstLine = errorMessage(error).split('\n')[0].slice(0, 160)
  const entry = Toolchains.loadFailures.get(extension) ?? { count: 0, reason: firstLine }
  entry.count += 1
  Toolchains.loadFailures.set(extension, entry)
}

/**
 * @param {string} label
 * @param {string} extension
 */
function skipAfterRepeatedLoadFailures(label, extension) {
  if (Settings.requireToolchains) return false
  const entry = Toolchains.loadFailures.get(extension)
  if (!entry || entry.count < Settings.loadFailureLimit) return false
  skip(label, `${extension.slice(1)} toolchain gave up ${entry.count} times under load, so the rest are not attempted: ${entry.reason}`)
  return true
}

/**
 * @param {string} label
 * @param {string} name
 * @param {string[]} extraTools
 */
async function canRun(label, name, extraTools = []) {
  const extension = path.extname(name)
  const tools = Toolchains.requiredTools[extension]
  if (!tools) {
    skip(label, `no execution checker for ${extension || 'descriptive example'}`)
    return false
  }
  if (skipAfterRepeatedLoadFailures(label, extension)) return false
  const missing = []
  for (const tool of [...tools, ...extraTools]) {
    if (!await isToolAvailable(tool)) missing.push(tool)
  }
  if (missing.length === 0) return true
  if (Settings.requireToolchains) {
    fail(label, new Error(`Required toolchain unavailable or failed its bounded startup probe: ${missing.join(', ')}`))
    return false
  }
  skip(label, `toolchain unavailable or failed its bounded startup probe: ${missing.join(', ')}`)
  return false
}

/**
 * @param {string} root
 * @param {string} name
 * @param {string} code
 */
function writeSnippetFile(root, name, code) {
  const isSafe = !path.isAbsolute(name) && !name.split(/[\\/]/).includes('..')
  assert.ok(isSafe, `Unsafe snippet filename: ${name}`)
  const file = path.join(root, name)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, code)
}

/** @param {string} root */
function emptyStagingDirectory(root) {
  mkdirSync(root, { recursive: true })
  for (const entry of readdirSync(root)) {
    if (entry !== '.tachyon') rmSync(path.join(root, entry), { recursive: true, force: true })
  }
  utimesSync(root, new Date(), new Date())
}

/**
 * @param {string} label
 * @param {(root: string) => Promise<void>} operation
 */
async function inStagingDirectory(label, operation) {
  const directoryName = createHash('sha256').update(label).digest('hex').slice(0, 32)
  const root = path.join(Settings.scratchRoot, directoryName)
  assert.ok(!Staging.activeDirectories.has(root), `Two steps share one staging directory: ${label}`)
  Staging.activeDirectories.add(root)
  try {
    emptyStagingDirectory(root)
    await operation(root)
  } catch (error) {
    fail(label, error)
  } finally {
    Staging.activeDirectories.delete(root)
  }
}

/**
 * @param {string} label
 * @param {string} root
 * @param {string[]} work
 * @param {() => Promise<unknown>} operation
 * @returns {Promise<string>}
 */
async function unlessPassedBefore(label, root, work, operation) {
  const key = Settings.cache.key(root, work)
  if (Settings.cache.hasPassed(key)) return ' (cached)'
  await operation()
  Settings.cache.rememberPass(key, label)
  return ''
}

/** @param {string} instance */
function serviceClassName(instance) {
  return `${instance[0].toUpperCase()}${instance.slice(1)}Service`
}

/** @param {string} name */
function instanceName(name) {
  const stem = path.basename(name).replace(/Service\.[a-z]+$/, '')
  return stem[0].toLowerCase() + stem.slice(1)
}

/** @param {string} instance */
function placeholderServiceCode(instance) {
  return `@Service\nexport default class ${serviceClassName(instance)} {}\n`
}

/** @param {string} template */
function servicesNamedByHandlers(template) {
  const names = new Set()
  for (const match of template.matchAll(Patterns.handlerService)) names.add(match[1])
  return names
}

/**
 * @param {string} root
 * @param {Snippet[]} files
 */
function addPlaceholderComponents(root, files) {
  const missing = []
  for (const name of ['product/card', 'sales/chart', 'heavy/editor', 'static/note']) {
    const exists = files.some(file => file.name === `client/components/${name}/tac.html`)
    if (!exists) missing.push(name)
  }
  for (const name of missing) {
    writeSnippetFile(root, `client/components/${name}/tac.html`, '<article><h3>{placeholder.name}</h3><slot /></article>')
  }
  if (missing.length === 0) return
  const placeholderService = "@Service\nexport default class PlaceholderService { name = '' }\n"
  writeSnippetFile(root, 'client/services/PlaceholderService.js', placeholderService)
}

/** @param {Snippet[]} clientFiles */
function tacLanguagesIn(clientFiles) {
  const extensions = new Set()
  for (const file of clientFiles) {
    const extension = path.extname(file.name)
    if (Languages.tacSourceExtensions.has(extension)) extensions.add(extension)
  }
  return [...extensions]
}

/** @param {Snippet[]} clientFiles */
function extensionsToBundle(clientFiles) {
  const tacLanguages = tacLanguagesIn(clientFiles)
  if (Settings.onlyLanguage) return tacLanguages.filter(extension => extension === `.${Settings.onlyLanguage}`)
  if (tacLanguages.length > 0) return tacLanguages
  return ['']
}

/**
 * @param {Snippet[]} clientFiles
 * @param {string} extension
 */
function filesForTacLanguage(clientFiles, extension) {
  if (!extension) return clientFiles
  const sameLanguageCode = clientFiles
    .filter(file => path.extname(file.name) === extension)
    .map(file => file.code)
    .join('\n')
  return clientFiles.filter(file => {
    const fileExtension = path.extname(file.name)
    const realmImportPath = `/${file.name.split('/').slice(1).join('/')}`
    if (sameLanguageCode.includes(realmImportPath)) return true
    return !Languages.tacSourceExtensions.has(fileExtension) || fileExtension === extension
  })
}

/**
 * @param {string} label
 * @param {Snippet[]} files
 */
async function bundleProject(label, files, withPlaceholders = false, webOnly = false) {
  const clientFiles = files.filter(file => !file.name.startsWith('server/'))
  for (const extension of extensionsToBundle(clientFiles)) {
    const nativeTarget = webOnly ? undefined : Languages.nativeTargets[extension]
    const languageTag = extension ? ` [${extension.slice(1)}]` : ''
    await inStagingDirectory(`${label}${languageTag}`, async root => {
      const selectedFiles = filesForTacLanguage(clientFiles, extension)
      for (const file of selectedFiles) writeSnippetFile(root, file.name, file.code)
      if (withPlaceholders) addPlaceholderComponents(root, selectedFiles)
      const bundleArguments = ['bundle', root]
      if (nativeTarget) bundleArguments.push('--target', nativeTarget, '--native')
      const cached = await unlessPassedBefore(label, root, bundleArguments, () => runCommand(Settings.binary, bundleArguments))
      const note = nativeTarget ? ' (native source staging only)' : ''
      pass(`${label}${languageTag}${note}${cached}`, 'bundle')
    })
  }
}

/**
 * @param {string} label
 * @param {string} template
 * @param {Snippet[]} extraFiles
 */
async function bundleTemplate(label, template, extraFiles = [], webOnly = false) {
  const files = [
    { name: 'client/pages/tac.html', code: '<guide-view />\n' },
    { name: 'client/components/guide/view/tac.html', code: template },
    ...extraFiles,
  ]
  const shownServices = new Set(extraFiles.filter(file => isService(file.name)).map(file => instanceName(file.name)))
  for (const instance of servicesNamedByHandlers(template)) {
    if (shownServices.has(instance)) continue
    files.push({ name: `client/services/${serviceClassName(instance)}.js`, code: placeholderServiceCode(instance) })
  }
  await bundleProject(label, files, false, webOnly)
}

/**
 * @param {string} label
 * @param {Snippet[]} files
 */
async function bundleServices(label, files, webOnly = false) {
  const serviceFiles = files.filter(file => isService(file.name))
  const instances = [...new Set(serviceFiles.map(file => instanceName(file.name)))]
  const placeholders = instances.map(instance => `{${instance}}`)
  await bundleTemplate(label, `<main>${placeholders.join(' ')}</main>`, files, webOnly)
}

/** @param {Snippet[]} files */
function testUnitExtensions(files) {
  const extensions = new Set()
  for (const file of files) {
    if (isTestUnit(file.name)) extensions.add(path.extname(file.name))
  }
  return [...extensions]
}

/**
 * @param {string} label
 * @param {string} extension
 */
async function canRunTestUnits(label, extension) {
  if (extension !== '.swift') return canRun(label, `unit${extension}`)
  if (skipAfterRepeatedLoadFailures(label, extension)) return false
  if (await isToolAvailable('swiftc')) return true
  skip(label, 'toolchain unavailable or failed its bounded startup probe: swiftc')
  return false
}

/**
 * @param {string} label
 * @param {Snippet[]} files
 * @param {Snippet[]} earlierSteps
 */
async function runTestUnits(label, files, earlierSteps) {
  const coverage = earlierSteps.length > 0 ? ['--coverage'] : []
  for (const extension of testUnitExtensions(files)) {
    if (!isSelectedLanguage(`unit${extension}`)) continue
    const stepLabel = `${label} [${extension.slice(1)}]`
    if (!await canRunTestUnits(stepLabel, extension)) continue
    await inStagingDirectory(stepLabel, async root => {
      for (const file of [...earlierSteps, ...files]) {
        if (path.extname(file.name) === extension) writeSnippetFile(root, file.name, file.code)
      }
      const testArguments = ['test', root, ...coverage]
      const cached = await unlessPassedBefore(stepLabel, root, testArguments, async () => {
        const output = await runCommand(Settings.binary, testArguments, { timeout: 300_000 })
        assert.match(output, /^\d+ passed, 0 failed\.$/m, output)
        assert.doesNotMatch(output, /^No test units found/m, output)
      })
      pass(`${stepLabel}${cached}`, 'unit')
    })
  }
}

/**
 * @param {string} label
 * @param {string} code
 */
async function bundleServiceClass(label, code) {
  const name = code.match(/export default class ([A-Z]\w*Service)\b/)?.[1]
  if (!name) {
    skip(label, 'class excerpt without a <Name>Service default export; not a complete service file')
    return
  }
  const excerptManifest = { name: 'client/manifest.json', code: Patterns.excerptManifest }
  await bundleServices(label, [{ name: `client/services/${name}.js`, code }, excerptManifest])
}

/**
 * @param {string} code
 * @returns {{method:string, path:string}[]}
 */
function controllerRoutes(code) {
  const classMatch = code.match(Patterns.classRoute)
  assert.ok(classMatch, 'A controller needs a RequestRoute annotation on its class')
  const classPath = classMatch[1] ?? ''
  const routes = []
  for (const match of code.matchAll(Patterns.methodRoute)) {
    const method = match[1].toLowerCase() === 'stream' ? 'GET' : match[1].toUpperCase()
    const fullPath = `${classPath}${match[2] ?? ''}`.replace(Patterns.pathVariable, '42')
    routes.push({ method, path: fullPath || '/' })
  }
  assert.ok(routes.length > 0, 'A controller needs at least one route method, such as GetRoute')
  return routes
}

/** @param {string} code */
function isStreamingHandler(code) {
  return /\byield(?:\s+return)?\b/.test(code)
}

/** @param {string} code */
function usesRelay(code) {
  return /(?:\b[Rr]elay|Yon(?:::|\.)Relay?)\s*\(/.test(code)
}

/** @param {Response} response */
async function readBodyWithLimit(response) {
  const reader = response.body?.getReader()
  if (!reader) return ''
  let bytes = 0
  let text = ''
  const decoder = new TextDecoder()
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      bytes += value.byteLength
      assert.ok(bytes <= Limits.responseBytes, 'Snippet HTTP response exceeds 64KiB')
      text += decoder.decode(value, { stream: true })
    }
    return text + decoder.decode()
  } finally {
    await reader.cancel().catch(() => {})
  }
}

/**
 * @param {string} method
 * @param {boolean} streaming
 * @param {Record<string, any>} schema
 */
function requestOptionsFor(method, streaming, schema) {
  assert.ok(schema.methods?.[method], `The OPTIONS schema lists no ${method} method`)
  const request = schema.methods[method].request ?? {}
  const models = schema.models ?? {}
  /** @type {{method:string, headers:Record<string,string>, body?:string}} */
  const options = {
    method,
    headers: { accept: streaming ? 'text/event-stream' : 'application/json', 'content-type': 'application/json' },
  }
  for (const [name, value] of SnippetRequest.textValues(request.headers, models)) options.headers[name.toLowerCase()] = value
  const cookies = SnippetRequest.textValues(request.cookies, models).map(([name, value]) => `${name}=${encodeURIComponent(value)}`)
  if (cookies.length > 0) options.headers.cookie = cookies.join('; ')
  if (!['GET', 'HEAD', 'OPTIONS'].includes(method)) {
    options.body = JSON.stringify(SnippetRequest.body(request.body, models))
  }
  return options
}

/**
 * @param {string} routePath
 * @param {string} method
 * @param {Record<string, any>} schema
 */
function requestPathFor(routePath, method, schema) {
  const values = SnippetRequest.textValues(schema.methods?.[method]?.request?.query, schema.models ?? {})
  if (values.length === 0) return routePath
  return `${routePath}${routePath.includes('?') ? '&' : '?'}${new URLSearchParams(values)}`
}

/** @param {string} event */
function eventData(event) {
  const dataLines = []
  for (const line of event.split(/\r?\n/)) {
    if (line.startsWith('data:')) dataLines.push(line.slice(5).trim())
  }
  return dataLines.join('\n')
}

/**
 * @param {string} contentType
 * @param {string} body
 */
function checkEventStream(contentType, body) {
  assert.match(contentType, /^text\/event-stream/)
  const frames = body.split(/\r?\n\r?\n/)
  assert.ok(frames.some(event => /^event:\s*end$/m.test(event)), `Expected the end event: ${body}`)
  const events = frames.filter(event => /^data:/m.test(event) && !/^event:\s*end$/m.test(event))
  assert.ok(events.length >= 2, `Expected multiple SSE messages: ${body}`)
  for (const event of events) {
    assert.ok(!/^event:\s*error/m.test(event), event)
    const value = JSON.parse(eventData(event))
    assert.equal(typeof value.sku, 'string')
    assert.equal(typeof value.level, 'number')
  }
}

/** @param {string} body */
function checkHtmlBody(body) {
  assert.match(body, /\{not-a-template\}/)
  assert.match(body, /<if>literal<\/if>/)
  assert.doesNotMatch(body, /tachyon-view/)
}

/**
 * @param {string} body
 * @param {string} method
 * @param {((value: any) => void) | undefined} checkValue
 */
function checkJsonBody(body, method, checkValue) {
  const value = JSON.parse(body)
  assert.ok(!Object.hasOwn(value, 'error'), body)
  if (checkValue && method === 'GET') checkValue(value)
}

/**
 * @param {string} label
 * @param {(path: string, options?: RequestInit) => Promise<Response>} request
 * @param {string} routePath
 */
async function checkSchemaAnswer(label, request, routePath) {
  const response = await request(routePath, { method: 'OPTIONS' })
  const body = await readBodyWithLimit(response)
  assert.equal(response.status, 200, `OPTIONS returned ${response.status}: ${body}`)
  const schema = JSON.parse(body)
  assert.equal(typeof schema.path, 'string', `OPTIONS must describe the route: ${body}`)
  assert.equal(typeof schema.methods, 'object', `OPTIONS must list the route methods: ${body}`)
  pass(`${label} OPTIONS schema`, 'http')
  return schema
}

/**
 * @param {string} label
 * @param {Snippet[]} files
 * @param {Snippet} handler
 * @param {(value: any) => void} [checkValue]
 * @param {ExpectedError} [expectedError]
 */
async function executeHandler(label, files, handler, checkValue, expectedError) {
  if (!isSelectedLanguage(handler.name)) return
  const handlerExtension = path.extname(handler.name)
  const sameLanguageFiles = files.filter(file => path.extname(file.name) === handlerExtension)
  const source = sameLanguageFiles.map(file => file.code).join('\n')
  const extraTools = [...(usesRelay(source) ? ['ruby'] : []), ...gatewayTools(sameLanguageFiles)]
  if (!await canRun(label, handler.name, extraTools)) return
  const streaming = isStreamingHandler(handler.code)
  const socket = handler.code.match(Patterns.socketRoute)
  await inStagingDirectory(label, async root => {
    for (const file of files) writeSnippetFile(root, file.name, file.code)
    await withServer(Settings.binary, ['serve', root, '--no-watch', '--port', '0'], async request => {
      if (socket) {
        await checkSocket(label, request, socket[1] || '/')
        return
      }
      const routes = controllerRoutes(handler.code)
      const schemas = new Map()
      const identity = files.some(file => Patterns.identityProviderPath.test(file.name))
      let refused = 0
      for (const { method, path: routePath } of routes) {
        if (!schemas.has(routePath)) schemas.set(routePath, await checkSchemaAnswer(label, request, routePath))
        const schema = schemas.get(routePath)
        const options = requestOptionsFor(method, streaming, schema)
        if (identity) refused += await refusedWithoutIdentity(request, requestPathFor(routePath, method, schema), options)
        const response = await request(requestPathFor(routePath, method, schema), options)
        const body = await readBodyWithLimit(response)
        if (expectedError) {
          checkHandledError(expectedError, schemas.get(routePath), method, response.status, body)
          pass(`${label} ${method} error handler`, 'http')
          continue
        }
        assert.ok(response.status >= 200 && response.status < 300, `${method} returned ${response.status}: ${body}`)
        const contentType = response.headers.get('content-type') ?? ''
        if (streaming) checkEventStream(contentType, body)
        else if (contentType.toLowerCase().startsWith('text/html')) checkHtmlBody(body)
        else checkJsonBody(body, method, checkValue)
        pass(`${label} ${method}${streaming ? ' SSE' : ''}`, 'http')
      }
      if (identity) {
        assert.ok(refused > 0, 'A guarded example must answer 401 to a caller who is not signed in')
        pass(`${label} 401 without identity`, 'http')
      }
    })
  })
}

/**
 * @param {(path: string, options?: RequestInit) => Promise<Response>} request
 * @param {string} requestPath
 * @param {{method:string, headers:Record<string,string>, body?:string}} options
 */
async function refusedWithoutIdentity(request, requestPath, options) {
  const anonymous = await request(requestPath, options)
  await readBodyWithLimit(anonymous)
  options.headers.authorization = Settings.identityToken
  return anonymous.status === 401 ? 1 : 0
}

/**
 * @param {string} label
 * @param {(path: string, options?: RequestInit) => Promise<Response>} request
 * @param {string} routePath
 */
async function checkSocket(label, request, routePath) {
  const plain = await request(routePath)
  assert.equal(plain.status, 426, `A socket path answers a plain request with 426, not ${plain.status}`)
  const url = new URL(plain.url)
  url.protocol = 'ws:'
  const first = await SocketProbe.firstMessage(url)
  checkJsonData(`${label} WebSocket message`, first)
}

/**
 * @param {ExpectedError} expected
 * @param {any} schema
 * @param {string} method
 * @param {number} status
 * @param {string} body
 */
function checkHandledError(expected, schema, method, status, body) {
  assert.equal(status, expected.status, `${method} must answer through its error handler: ${body}`)
  assert.deepEqual(JSON.parse(body), expected.value, 'The error handler answer must be the returned model')
  const errors = /** @type {{ type: unknown, status: number }[]} */ (schema.methods[method]?.errors ?? [])
  const listed = errors.some(error => String(error.type).replace(/^[&*]/, '').endsWith(expected.type) && error.status === expected.status)
  assert.ok(listed, `OPTIONS must list ${expected.type} ${expected.status}: ${JSON.stringify(errors)}`)
}

/**
 * @param {string} label
 * @param {Snippet} middleware
 */
async function executeMiddleware(label, middleware) {
  if (!isSelectedLanguage(middleware.name)) return
  if (!await canRun(label, middleware.name)) return
  await inStagingDirectory(label, async root => {
    writeSnippetFile(root, middleware.name, middleware.code)
    writeSnippetFile(root, 'server/controllers/PrivateController.js', Patterns.middlewareController)
    await withServer(Settings.binary, ['serve', root, '--no-watch', '--port', '0'], async request => {
      const refused = await request('/private')
      const refusedBody = await readBodyWithLimit(refused)
      assert.equal(refused.status, 401, `before must refuse a request without the token: ${refusedBody}`)
      const allowed = await request('/private', { headers: { authorization: 'let-me-in' } })
      const allowedBody = await readBodyWithLimit(allowed)
      assert.equal(allowed.status, 200, `before must let the token through: ${allowedBody}`)
      assert.deepEqual(JSON.parse(allowedBody), { secret: 42 })
      assert.equal(allowed.headers.get('x-served-by'), 'tachyon', 'after must add its header')
      pass(`${label} before and after`, 'http')
    })
  })
}

/**
 * @param {string} extension
 * @param {string} file
 * @returns {[string, string[]]}
 */
function syntaxCheckCommand(extension, file) {
  if (extension === '.py') return ['python3', ['-m', 'py_compile', file]]
  if (extension === '.rb') return ['ruby', ['-c', file]]
  if (extension === '.lua') return ['luac', ['-p', file]]
  if (extension === '.php') return ['php', ['-l', file]]
  if (extension === '.dart') return ['dart', ['analyze', '--fatal-warnings', file]]
  if (extension === '.go') return ['gofmt', ['-e', '-l', file]]
  return [Settings.bun, ['build', '--no-bundle', file, '--outfile', 'parsed.js']]
}

/**
 * @param {string} label
 * @param {string} name
 * @param {string} code
 */
async function checkSyntax(label, name, code) {
  if (!await canRun(label, name)) return
  const extension = path.extname(name)
  if (['.java', '.groovy', '.kt', '.cs', '.fs', '.rs'].includes(extension)) {
    skip(label, 'standalone layer fragment; full published language suites are compiled and executed separately')
    return
  }
  await inStagingDirectory(label, async root => {
    const file = `snippet${extension === '.js' ? '.mjs' : extension}`
    const source = extension === '.go' && !/^package\s/m.test(code) ? `package main\n\n${code}` : code
    writeSnippetFile(root, file, source)
    const [command, args] = syntaxCheckCommand(extension, file)
    const output = await runCommand(command, args, { cwd: root, timeout: 30_000 })
    if (extension === '.go') assert.equal(output.trim(), '', `${label} needs gofmt`)
    pass(label, 'syntax')
  })
}

/**
 * @param {string} label
 * @param {string} text
 */
function checkJsonData(label, text) {
  try {
    JSON.parse(text)
    pass(label, 'data')
  } catch (error) {
    fail(label, error)
  }
}

/**
 * @param {Feature} feature
 * @param {Snippet} file
 * @param {boolean} hasPage
 */
async function verifyFeatureFile(feature, file, hasPage) {
  const label = `${feature.id} / ${file.name}`
  if (hasPage && !file.name.startsWith('server/')) {
    skip(label, 'compiled within the page project')
    return
  }
  if (Patterns.controllerPath.test(file.name)) {
    const serverFiles = feature.files.filter(other => other.name.startsWith('server/'))
    await executeHandler(label, filesForHandlerLanguage(serverFiles, file), file)
    return
  }
  if (Patterns.componentViewPath.test(file.name)) {
    await bundleTemplate(label, file.code, feature.files.filter(other => isService(other.name)))
    return
  }
  if (isService(file.name)) {
    skip(label, 'compiled with the view that names it')
    return
  }
  if (/\.json$/.test(file.name) || file.name === '.tachyonrc') {
    checkJsonData(label, file.code.replace(/^\s*\/\/.*$/gm, ''))
    return
  }
  if (Patterns.middlewareFile.test(file.name)) {
    await executeMiddleware(label, file)
    return
  }
  if (isRunWithItsController(feature, file)) {
    skip(label, 'staged and run with the controller in its language')
    return
  }
  if (Patterns.sharedFile.test(file.name)) {
    const probe = SharedModuleProbe.controller(file)
    await executeHandler(label, [probe, file], probe, value => assert.deepEqual(value, SharedModuleProbe.expected(file)))
    return
  }
  if (/\.(js|ts|py|php|rb)$/.test(file.name) || file.name.startsWith('server/')) {
    await checkSyntax(label, file.name, file.code)
    return
  }
  skip(label, 'descriptive output, shell command, or stylesheet; not an executable snippet')
}

/**
 * @param {Feature} feature
 * @param {Snippet} file
 */
function isRunWithItsController(feature, file) {
  if (!file.name.startsWith('server/') || Patterns.sharedFile.test(file.name)) return false
  const extension = path.extname(file.name)
  return feature.files.some(other => Patterns.controllerPath.test(other.name) && path.extname(other.name) === extension)
}

/** @param {Feature} feature */
async function verifyFeature(feature) {
  const hasPage = feature.files.some(isPageFile)
  if (hasPage) {
    const usesPlaceholders = feature.verify === 'tac-component' || feature.files.some(file => /<product-card/.test(file.code))
    await bundleProject(feature.id, feature.files, usesPlaceholders, feature.target === 'web')
  }
  for (const file of feature.files) await verifyFeatureFile(feature, file, hasPage)
}

async function verifyFeatureSnippets() {
  console.log('== feature snippets')
  const features = SiteData.features.filter(feature => !Settings.onlyFeature || feature.id === Settings.onlyFeature)
  assert.ok(features.length > 0, `No feature matched: ${Settings.onlyFeature}`)
  await runInParallel(features.map(feature => () => verifyFeature(feature)))
}

/** @param {{id:string,files:Snippet[]}} entry */
async function verifyYonLanguageEntry(entry) {
  const handler = entry.files.find(file => Patterns.controllerPath.test(file.name))
  assert.ok(handler, `No controller in ${entry.id}`)
  const label = `yon/${entry.id} Handler suite (${entry.files.length} files)`
  await executeHandler(label, entry.files, handler, value => {
    assert.ok(Array.isArray(value.orders) && value.orders.length > 0, `Handler must reach its nonempty repository data: ${JSON.stringify(value)}`)
    assert.ok(JSON.stringify(value.orders).includes('anvil'), 'The published repository fixture must reach the HTTP response')
  })
}

async function verifyLanguageSnippets() {
  console.log('== language snippets')
  const basketPage = { name: 'client/pages/tac.html', code: '<basket-tally hydrate="load" />\n' }
  await bundleProject('tac / basket-tally', [basketPage, ...SiteData.languages.tac.files])
  await runInParallel(SiteData.languages.yon.entries.map(entry => () => verifyYonLanguageEntry(entry)))
}

/**
 * @param {string} slug
 * @param {GuideSection} section
 */
function expectedGuideValue(slug, section) {
  if (slug === 'annotations' && section.heading === 'Typed model mapping: Yon') {
    return (/** @type {Record<string,unknown>} */ value) => assert.deepEqual(value, {
      id: 7, name: 'Anvil', detail: { name: 'detail' }, items: [{ name: 'detail' }], kind: 'product',
    }, 'Typed converters must project nested models and lists, drop secrets and preserve explicit overrides')
  }
  if (slug === 'application-configuration' && section.heading === 'Access from every Yon language') {
    return (/** @type {Record<string,unknown>} */ value) =>
      assert.deepEqual(value, { pageSize: 20, port: 0 }, 'Each language must bind the same private configuration snapshot')
  }
  if (slug === 'annotations' && section.heading === 'Transactions: Yon') {
    return (/** @type {{commands:string[],settled:{open:{transaction?:string}[]}}} */ value) => {
      const storeLogged = value.commands.some(command => command.startsWith('find@'))
      const expected = storeLogged ? ['begin', 'find@t1', 'count@t1', 'commit:t1'] : ['begin', 'commit:t1']
      assert.deepEqual(value.commands.slice(0, expected.length), expected, 'The injected store must join the service transaction')
      if (!storeLogged) assert.equal(value.settled.open[0].transaction, 't1', 'The injected store must query inside the service transaction')
    }
  }
  if (slug === 'application-events' && section.heading.startsWith('Yon')) {
    return (/** @type {Record<string,unknown>} */ value) => assert.deepEqual(value, { value: 7 }, 'The listener must keep the published ProbeReady')
  }
  if (slug === 'project-folders' && section.heading.startsWith('Models')) {
    return (/** @type {Record<string,unknown>} */ value) => assert.deepEqual(value, { title: 'Models hold types', pages: 3 })
  }
  if (slug === 'project-folders') {
    return (/** @type {Record<string,unknown>} */ value) => assert.deepEqual(value, { amount: 42 })
  }
  return undefined
}

/**
 * @param {string} slug
 * @param {GuideSection} section
 * @returns {ExpectedError | undefined}
 */
function expectedGuideError(slug, section) {
  if (slug === 'error-handlers' && section.heading.startsWith('Error handlers')) {
    return { status: 404, type: 'OrderNotFound', value: { code: 'order-not-found' } }
  }
  return undefined
}

/**
 * @param {string} slug
 * @param {GuideSection} section
 * @param {Snippet[]} files
 */
async function verifyGuideFiles(slug, section, files) {
  const hasModules = hasRoleFiles(files)
  for (const file of files) {
    if (file.name.startsWith('client/')) continue
    const label = `${slug} / ${section.heading} / ${file.name}`
    if (Patterns.controllerPath.test(file.name)) {
      await executeHandler(label, filesForHandlerLanguage(files, file), file, expectedGuideValue(slug, section), expectedGuideError(slug, section))
    } else if (file.name.endsWith('.json')) {
      checkJsonData(label, file.code)
    } else if (!hasModules) {
      skip(label, 'guide excerpt without a complete execution fixture')
    }
  }
}

/**
 * @param {string} label
 * @param {string} code
 */
async function verifyConfigurationHandler(label, code) {
  const handler = { name: 'server/controllers/ConfigurationController.js', code }
  const settings = { server: { host: '127.0.0.1', port: 0 }, catalog: { page: { size: 25 } } }
  const application = { name: 'server/application.json', code: JSON.stringify(settings) }
  await executeHandler(label, [application, handler], handler, value => {
    assert.deepEqual(value, { pageSize: 25, port: 0 }, 'Configuration example must bind the private runtime snapshot')
  })
}

/**
 * @param {string} slug
 * @param {GuideSection} section
 */
async function verifyGuideCode(slug, section) {
  const label = `${slug} / ${section.heading}`
  const code = section.code
  if (slug === 'application-configuration' && code?.includes('@RequestRoute')) {
    await verifyConfigurationHandler(label, code)
    return
  }
  if (code && Patterns.serviceExcerpt.test(code.trim())) {
    await bundleServiceClass(label, code)
    return
  }
  if (code) skip(label, 'guide command, layout, or excerpt; not a complete source file')
}

/**
 * @param {string} slug
 * @param {GuideSection} section
 * @param {Snippet[]} earlierSteps
 */
async function verifyGuideSection(slug, section, earlierSteps) {
  const files = section.files ?? []
  const label = `${slug} / ${section.heading}`
  if (slug === 'annotations' && section.heading === 'Configuration binding: Tac') {
    await verifyTacConfiguration(label, files)
    return
  }
  const hasTestUnits = files.some(file => isTestUnit(file.name))
  if (hasTestUnits) await runTestUnits(label, files, earlierSteps)
  else if (files.some(isPageFile)) await bundleProject(label, files, false, section.target === 'web')
  else if (files.some(file => isService(file.name))) await bundleServices(label, files, section.target === 'web')
  if (!hasTestUnits) await verifyGuideFiles(slug, section, files)
  await verifyGuideCode(slug, section)
}

/**
 * @param {string} label
 * @param {Snippet[]} files
 */
async function verifyTacConfiguration(label, files) {
  for (const extension of extensionsToBundle(files)) {
    const stepLabel = `${label} [${extension.slice(1)}]`
    if (!await canRunTestUnits(stepLabel, extension)) continue
    await inStagingDirectory(stepLabel, async root => {
      for (const file of filesForTacLanguage(files, extension)) writeSnippetFile(root, file.name, file.code)
      if (!await hasConfigurationWebTarget(extension)) {
        await verifyNativeConfiguration(stepLabel, root, files, extension)
        return
      }
      await runCommand(Settings.binary, ['bundle', root], { timeout: 300_000 })
      pass(stepLabel, 'bundle')
      await checkConfigurationBrowser(Settings.binary, root)
      pass(`${stepLabel} (browser runtime)`, 'browser')
    })
  }
}

/** @param {string} extension */
async function hasConfigurationWebTarget(extension) {
  if (extension === '.swift') return Boolean(process.env.TAC_SWIFT_WASM_SDK) && await isToolAvailable(process.env.TAC_SWIFT_WASM_COMPILER ?? 'swiftc')
  if (extension === '.kt') return Boolean(process.env.TAC_KOTLIN_WASM_STDLIB) && await isToolAvailable('kotlinc-js')
  if (extension === '.java') return Boolean(process.env.TAC_TEAVM_HOME) && await isToolAvailable('javac')
  if (extension === '.rs') {
    const targets = await runCommand('rustup', ['target', 'list', '--installed'])
    return targets.split('\n').includes('wasm32-unknown-unknown')
  }
  if (extension === '.cs' || extension === '.fs') {
    const workloads = await runCommand('dotnet', ['workload', 'list'])
    return /^wasm-tools\s/m.test(workloads)
  }
  return true
}

/**
 * @param {string} label
 * @param {string} root
 * @param {Snippet[]} files
 * @param {string} extension
 */
async function verifyNativeConfiguration(label, root, files, extension) {
  const service = files.find(file => file.name === `client/services/SettingsService${extension}`)
  assert.ok(service, 'The native configuration example must publish its SettingsService')
  const bundleArguments = ['bundle', root, '--target', Languages.nativeTargets[extension], '--native']
  const bundled = await unlessPassedBefore(label, root, bundleArguments, () => runCommand(Settings.binary, bundleArguments))
  pass(`${label} (native source staging)${bundled}`, 'bundle')
  for (const file of configurationTestFiles(extension, service.code)) writeSnippetFile(root, file.name, file.code)
  const tested = await unlessPassedBefore(label, root, ['test', root], async () => {
    const output = await runCommand(Settings.binary, ['test', root], { timeout: 300_000 })
    assert.match(output, /^1 passed, 0 failed\.$/m, output)
  })
  pass(`${label} (native runtime)${tested}`, 'unit')
}

/**
 * @param {Record<string, any>} base
 * @param {Record<string, any>} overlay
 * @returns {Record<string, any>}
 */
function mergeConfiguration(base, overlay) {
  const result = { ...base }
  for (const [key, value] of Object.entries(overlay)) {
    const isNestedObject = Boolean(value) && typeof value === 'object' && !Array.isArray(value)
    if (isNestedObject) result[key] = mergeConfiguration(base[key] ?? {}, value)
    else result[key] = value
  }
  return result
}

async function verifyConfigurationWorkedExample() {
  if (!isSelectedLanguage('ProductsController.ts')) return
  const guideText = readFileSync(path.join(Settings.repo, 'docs/APPLICATION_CONFIGURATION.md'), 'utf8')
  const workedExample = guideText.split('## A complete worked example')[1]
  assert.ok(workedExample, 'The application configuration worked example must remain verifiable')
  const blocks = [...workedExample.matchAll(/^### `?((?:server|client)\/[^`\n]+)`?\n\n```(?:json|typescript)\n([\s\S]*?)\n```/gm)]
  const snippets = new Map(blocks.map(block => [block[1], block[2]]))
  assert.equal(snippets.size, blocks.length, 'Worked example file names must be unique')
  assert.deepEqual([...snippets.keys()], [
    'server/application.json',
    'server/application-production.json',
    'server/helpers/CatalogProperties.ts',
    'server/helpers/Endpoint.ts',
    'server/controllers/ProductsController.ts',
    'server/setups/ProductionSetup.ts',
  ], 'The worked example must publish every source as its own named file')
  const base = JSON.parse(snippets.get('server/application.json') ?? '')
  const production = JSON.parse(snippets.get('server/application-production.json') ?? '')
  const application = mergeConfiguration(base, production)
  const sources = [...snippets].filter(([name]) => name.endsWith('.ts')).map(([name, code]) => ({ name, code }))
  const handler = sources.find(file => file.name === 'server/controllers/ProductsController.ts')
  assert.ok(handler, 'The worked example must declare a route controller')
  const label = 'application-configuration / complete TypeScript worked example'
  if (await isPortListening(6379)) {
    const files = [
      { name: 'server/application.json', code: JSON.stringify(application) },
      ...sources,
    ]
    await executeHandler(label, files, handler, value => assert.deepEqual(value, { pageSize: 20, port: 0 }))
    return
  }
  if (Settings.requireToolchains) fail(label, new Error('Redis is not listening on 127.0.0.1:6379'))
  else skip(label, 'Redis is not listening on 127.0.0.1:6379; the example initialises its Redis datasource at startup')
}

/** @param {string} scope */
function onlyTopicFor(scope) {
  if (scope === 'configuration') return 'application-configuration'
  if (scope === 'layout') return 'project-folders'
  if (scope === 'annotations') return 'annotations'
  return undefined
}

async function verifyGuideSnippets() {
  console.log('== guide snippets')
  const onlyTopic = onlyTopicFor(Settings.scope)
  /** @type {(() => Promise<void>)[]} */
  const steps = []
  for (const [slug, topic] of Object.entries(SiteData.docs.topics)) {
    if (onlyTopic && slug !== onlyTopic) continue
    /** @type {Snippet[]} */
    const tutorialFiles = []
    for (const section of topic.sections ?? []) {
      const earlierSteps = [...tutorialFiles]
      if (Patterns.tutorials.has(slug)) tutorialFiles.push(...section.files ?? [])
      if (Settings.onlySection && section.heading !== Settings.onlySection) continue
      steps.push(() => verifyGuideSection(slug, section, earlierSteps))
    }
  }
  assert.ok(!Settings.onlySection || steps.length > 0, `No section matched: ${Settings.onlySection}`)
  await runInParallel(steps)
  if (!Settings.onlySection) await verifyConfigurationWorkedExample()
}

/** @param {number} milliseconds */
function seconds(milliseconds) {
  return `${(milliseconds / 1000).toFixed(1)}s`
}

function printTimingSummary() {
  const timings = Progress.timings
  /** @type {Map<string, number>} */
  const totalByKind = new Map()
  for (const entry of timings) totalByKind.set(entry.kind, (totalByKind.get(entry.kind) ?? 0) + entry.ms)
  const kindTotals = [...totalByKind.entries()].sort((left, right) => right[1] - left[1])
  const kindText = kindTotals.map(([kind, milliseconds]) => `${kind} ${seconds(milliseconds)}`).join(', ')
  console.log(`\ntime: ${seconds(Date.now() - Progress.started)} total across ${timings.length} steps — ${kindText}`)
  /** @type {Map<string, number>} */
  const totalByLanguage = new Map()
  for (const entry of timings) {
    const language = labelExtension(entry.label).slice(1) || 'other'
    totalByLanguage.set(language, (totalByLanguage.get(language) ?? 0) + entry.ms)
  }
  const languageTotals = [...totalByLanguage.entries()].sort((left, right) => right[1] - left[1])
  console.log(`by language: ${languageTotals.map(([language, milliseconds]) => `${language} ${seconds(milliseconds)}`).join(', ')}`)
  const slowest = timings.sort((left, right) => right.ms - left.ms).slice(0, 12)
  const slowestLines = slowest.map(entry => `  ${seconds(entry.ms).padStart(7)}  ${entry.kind.padEnd(7)} ${entry.label}`)
  console.log(`slowest:\n${slowestLines.join('\n')}`)
}

function printResult() {
  const counts = Progress.counts
  const status = counts.failed ? 'FAIL' : 'PASS'
  console.log(`\n${status}: ${counts.bundle} bundles, ${counts.browser} browser probes, ${counts.http} HTTP method probes, ${counts.unit} test units, ${counts.syntax} syntax checks, ${counts.types} tsc projects, ${counts.data} JSON checks; ${counts.skipped} explicitly unexecuted examples; ${counts.failed} failures; ${Settings.cache.hits} steps passed before with the same inputs.`)
  process.exitCode = counts.failed ? 1 : 0
}

/**
 * @param {unknown} value
 * @param {Snippet[]} found
 * @returns {Snippet[]}
 */
function collectGoFiles(value, found) {
  if (Array.isArray(value)) {
    for (const item of value) collectGoFiles(item, found)
    return found
  }
  if (!value || typeof value !== 'object') return found
  const record = /** @type {Record<string, unknown>} */ (value)
  if (typeof record.name === 'string' && record.name.endsWith('.go') && typeof record.code === 'string') {
    found.push({ name: record.name, code: record.code })
  }
  for (const child of Object.values(record)) collectGoFiles(child, found)
  return found
}

async function verifyGoFormatting() {
  console.log('== gofmt over every published Go file')
  const label = 'published Go files'
  if (!await canRun(label, 'published.go')) return
  const files = collectGoFiles([SiteData.features, SiteData.languages, SiteData.docs], [])
  assert.ok(files.length > 0, 'The site data holds no Go files')
  await inStagingDirectory(label, async root => {
    files.forEach((file, index) => {
      const code = /^package\s/m.test(file.code) ? file.code : `package main\n\n${file.code}`
      writeSnippetFile(root, `${String(index).padStart(4, '0')}.go`, code)
    })
    const output = await runCommand('gofmt', ['-e', '-l', '.'], { cwd: root, timeout: 60_000 })
    const changed = output.split('\n').filter(Boolean).map(line => files[Number(path.basename(line, '.go'))]?.name ?? line)
    assert.deepEqual(changed, [], `gofmt would change these published Go files: ${changed.join(', ')}`)
    pass(`${label} (${files.length}) [gofmt]`, 'syntax')
  })
}

/**
 * @param {{ language: string, extension: string, check: string, tool: string, args: (file: string, root: string) => string[] }} syntax
 */
async function verifyContentSyntax(syntax) {
  const label = `${syntax.language} files in website/content`
  console.log(`== ${syntax.check} over every ${label}`)
  if (!await canRun(label, `content${syntax.extension}`)) return
  const entries = readdirSync(SiteContent.contentFolder, { recursive: true, withFileTypes: true })
  const files = entries
    .filter(entry => entry.isFile() && entry.name.endsWith(syntax.extension))
    .map(entry => path.relative(SiteContent.contentFolder, path.join(entry.parentPath, entry.name)))
    .sort()
  assert.ok(files.length > 0, `website/content holds no ${syntax.language} files`)
  await inStagingDirectory(label, async root => {
    const failed = []
    for (const file of files) {
      try {
        await runCommand(syntax.tool, syntax.args(file, root), { cwd: SiteContent.contentFolder, timeout: 30_000 })
      } catch (error) {
        failed.push(`${file}: ${error instanceof Error ? error.message : String(error)}`)
      }
    }
    assert.deepEqual(failed, [], `${syntax.check} rejects these files in website/content:\n${failed.join('\n')}`)
    pass(`${label} (${files.length}) [${syntax.check}]`, 'syntax')
  })
}

async function verifyRubySyntax() {
  await verifyContentSyntax({ language: 'Ruby', extension: '.rb', check: 'ruby -c', tool: 'ruby', args: file => ['-c', file] })
}

async function verifyPythonSyntax() {
  await verifyContentSyntax({
    language: 'Python',
    extension: '.py',
    check: 'python3 -m py_compile',
    tool: 'python3',
    args: (file, root) => ['-X', `pycache_prefix=${root}`, '-m', 'py_compile', file],
  })
}

/**
 * @param {string} extension
 * @returns {Promise<Snippet[]>}
 */
async function scaffoldTypeFiles(extension) {
  const language = extension.slice(1)
  /** @type {Snippet[]} */
  const files = []
  await inStagingDirectory(`tsc scaffold [${language}]`, async root => {
    const project = path.join(root, 'app')
    await runCommand(Settings.binary, ['init', project, '--tac', language, '--yon', language, '--skip-tools'], { timeout: 60_000 })
    for (const name of Patterns.typeScaffoldFiles) files.push({ name, code: readFileSync(path.join(project, name), 'utf8') })
  })
  return files
}

/**
 * @param {import('./type-check-projects.mjs').TypeCheckProject} project
 * @param {Snippet[]} scaffoldFiles
 */
async function typeCheckProject(project, scaffoldFiles) {
  await inStagingDirectory(`${project.label} [tsc]`, async root => {
    for (const file of [...scaffoldFiles, ...project.files]) writeSnippetFile(root, file.name, file.code)
    const work = ['types', root, 'tsc', ...project.realms]
    const cached = await unlessPassedBefore(project.label, root, work, async () => {
      await runCommand(Settings.binary, ['types', root], { timeout: 60_000 })
      for (const realm of project.realms) {
        await runCommand('tsc', ['-p', path.join(root, realm, 'jsconfig.json')], { cwd: root, timeout: 60_000 })
      }
    })
    pass(`${project.label} [tsc]${cached}`, 'types')
  })
}

async function verifyTypeChecks() {
  console.log('== tsc over every JavaScript and TypeScript example project')
  const label = 'JavaScript and TypeScript example projects'
  if (!await isToolAvailable('tsc')) {
    const missing = new Error('tsc is not installed; run bun install at the repository root')
    if (Settings.requireToolchains) fail(label, missing)
    else skip(label, missing.message)
    return
  }
  const filter = { name: Settings.onlyFeature ?? Settings.onlySection, language: Settings.onlyLanguage }
  const projects = TypeCheckProjects.select(
    TypeCheckProjects.collect(SiteData.features, SiteData.languages, SiteData.docs, Patterns.tutorials),
    filter,
  )
  if (projects.length === 0) {
    console.log('  --  0 tsc projects matched the filters; the tsc step has nothing to check')
    return
  }
  /** @type {Map<string, Snippet[]>} */
  const scaffolds = new Map()
  for (const extension of new Set(projects.map(project => project.extension))) {
    scaffolds.set(extension, await scaffoldTypeFiles(extension))
  }
  await runInParallel(projects.map(project => () => typeCheckProject(project, scaffolds.get(project.extension) ?? [])))
}

async function main() {
  const scope = Settings.scope
  console.log(`Selected CLI: ${Settings.binary}`)
  console.log(`Staging: ${Settings.scratchRoot} (compiled handlers are cached here between runs)`)
  console.log(`Concurrency: ${Settings.concurrency} of ${cpus().length} cores, load ${loadavg()[0].toFixed(1)}`)
  console.log((await runCommand(Settings.binary, ['--version'], { timeout: 10_000 })).trim())
  Settings.cache.fingerprint = await ToolchainFingerprint.read(Settings.repo, Settings.binary)
  Settings.cache.removeOldEntries()
  console.log(`Cache: ${Settings.cache.reads ? Settings.cache.folder : 'off (--no-cache); passes are still written'}`)
  if (scope === 'all' || scope === 'gofmt') await verifyGoFormatting()
  if (scope === 'all' || scope === 'ruby') await verifyRubySyntax()
  if (scope === 'all' || scope === 'python') await verifyPythonSyntax()
  if (scope === 'all' || scope === 'tsc') await verifyTypeChecks()
  if (scope === 'all' || scope === 'features') await verifyFeatureSnippets()
  if (scope === 'all' || scope === 'languages') await verifyLanguageSnippets()
  if (['all', 'guides', 'configuration', 'layout', 'annotations'].includes(scope)) await verifyGuideSnippets()
  printTimingSummary()
  printResult()
}

await main()
