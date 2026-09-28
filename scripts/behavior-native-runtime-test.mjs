import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

class NativeBehaviorTest {
  static root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  static cachePolicy = { name: 'products', ttlMs: 30000, maxBytes: 65536, maxEntries: 256 };
  static cacheTarget = { method: 'list', cacheable: { keyArgs: [0] }, pageable: { argument: 0, defaultSize: 20, maxSize: 100, sort: ['id'] } };
  static registry = JSON.stringify({ Catalog: { cache: NativeBehaviorTest.cachePolicy, methods: {
    list: { cacheable: { keyArgs: [0] }, pageable: { argument: 0, defaultSize: 20, maxSize: 100, sort: ['id'] } },
    remote: { httpExchange: { client: 'catalog', method: 'GET', path: '/products' } },
    update: { cachePut: { method: 'list', arguments: [1], target: NativeBehaviorTest.cacheTarget } },
    remove: { cacheEvict: { method: 'list', arguments: [0], target: NativeBehaviorTest.cacheTarget } },
    updateRemote: { cachePut: { method: 'list', arguments: [0], target: NativeBehaviorTest.cacheTarget }, httpExchange: { client: 'catalog', method: 'GET', path: '/products' } },
  } } });
  static mapping = JSON.stringify({ entity: 'Product', adapter: 'data', idField: 'id', table: 'products' });
  static configurationPolicies = {
    Config: {},
    ApplicationProperties: { configurationProperties: { prefix: 'TAC_APP_', fields: {
      label: { name: 'LABEL', type: 'string' },
      enabled: { name: 'ENABLED', type: 'boolean', default: false },
      workers: { name: 'WORKERS', type: 'integer', default: 4, min: 1, max: 64 },
      ratio: { name: 'RATIO', type: 'number', default: 1.5, min: 0, max: 10 },
    } } },
    InvalidProperties: { configurationProperties: { prefix: 'TAC_APP_', fields: { enabled: { name: 'ENABLED', type: 'boolean', default: 'true' } } } },
    GuardedConfig: { profiles: ['development', 'testing'], conditionalOnProperty: { name: 'TAC_CACHE_ENABLED', havingValue: 'true', matchIfMissing: false } },
    MissingConfig: { conditionalOnProperty: { name: 'TAC_CACHE_ENABLED', havingValue: 'true', matchIfMissing: true } },
  };
}

function parseArguments() {
  const argumentsList = process.argv.slice(2);
  const emittedIndex = argumentsList.indexOf('--emitted');
  const emittedPath = emittedIndex === -1 ? null : argumentsList[emittedIndex + 1];
  if (emittedIndex !== -1) argumentsList.splice(emittedIndex, 2);
  const languages = argumentsList;
  const selected = languages.length ? languages : process.platform === 'darwin' ? ['cs', 'swift'] : ['cs'];
  return { emittedPath, selected };
}

function run(directory, command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: directory, encoding: 'utf8', timeout: 180000, maxBuffer: 4 * 1024 * 1024, ...options });
  assert.equal(result.error, undefined, `${command}: ${result.error?.message}`);
  assert.equal(result.status, 0, `${command} ${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
  process.stdout.write(result.stdout);
}

function emitCode(language, template) {
  return template
    .replaceAll('__TAC_BEHAVIOR_MODULE__', JSON.stringify('a'.repeat(64)))
    .replaceAll('__TAC_BEHAVIOR_ENVIRONMENT__', JSON.stringify('tac'))
    .replaceAll('__TAC_BEHAVIOR_REGISTRY__', JSON.stringify(NativeBehaviorTest.registry))
    .replaceAll('__TAC_BEHAVIOR_CONFIGURATIONS__', ['Config', 'GuardedConfig', 'MissingConfig'].map(type => language === 'cs' ? `if (configuration == typeof(${type})) return ${type}.configure(execution);` : `if ObjectIdentifier(configuration) == ObjectIdentifier(${type}.self) { return ${type}.configure(execution) }`).join('\n'))
    .replaceAll('__TAC_BEHAVIOR_TYPES__', Object.entries(NativeBehaviorTest.configurationPolicies).map(([type, policy]) => language === 'cs' ? `if (type == typeof(${type})) return BehaviorJSON.Parse(${JSON.stringify(JSON.stringify(policy))});` : `if ObjectIdentifier(type) == ObjectIdentifier(${type}.self) { return try BehaviorJSON.parse(${JSON.stringify(JSON.stringify(policy))}) }`).join('\n'))
    .replaceAll('__TAC_BEHAVIOR_ENTITIES__', language === 'cs' ? `if (entity == typeof(Product)) return BehaviorJSON.Parse(${JSON.stringify(NativeBehaviorTest.mapping)});` : `if ObjectIdentifier(entity) == ObjectIdentifier(Product.self) { return try BehaviorJSON.parse(${JSON.stringify(NativeBehaviorTest.mapping)}) }`);
}

async function checkCSharp(directory, code, fixtures, emittedPath) {
  const version = spawnSync('dotnet', ['--version'], { encoding: 'utf8', timeout: 30000 });
  assert.equal(version.status, 0, version.stderr); const major = Number(version.stdout.trim().split('.')[0]);
  assert.ok(major >= 8, 'A supported .NET SDK is required');
  await writeFile(join(directory, 'Behavior.csproj'), `<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net${major}.0</TargetFramework><ImplicitUsings>disable</ImplicitUsings><Nullable>disable</Nullable><TreatWarningsAsErrors>true</TreatWarningsAsErrors><JsonSerializerIsReflectionEnabledByDefault>false</JsonSerializerIsReflectionEnabledByDefault>${emittedPath ? '<StartupObject>TachyonRoute0.EmittedBehaviorTests</StartupObject>' : ''}</PropertyGroup></Project>`);
  await writeFile(join(directory, 'Program.cs'), `namespace TachyonRoute0 {\n${code}\n${fixtures}\n}`);
  run(directory, 'dotnet', ['run', '--project', 'Behavior.csproj', '--nologo', '--disable-build-servers']);
}

async function checkSwift(directory, code, fixtures, emittedPath) {
  await writeFile(join(directory, 'Behavior.swift'), `import Foundation\nprivate enum TachyonRoute0 {\n${code}\n${fixtures}\n}\n@main struct Main { static func main() async throws { try await TachyonRoute0.${emittedPath ? 'EmittedBehaviorTests' : 'NativeBehaviorTests'}.run() } }`);
  run(directory, 'swiftc', ['-parse-as-library', '-warnings-as-errors', join(directory, 'Behavior.swift'), '-o', join(directory, 'behavior-swift')]);
  run(directory, join(directory, 'behavior-swift'), []);
}

async function checkLanguage(directory, language, emittedPath) {
  assert.ok(['cs', 'swift'].includes(language), `Unknown language ${language}`);
  const stem = language === 'cs' ? 'csharp' : 'swift';
  const template = await readFile(join(NativeBehaviorTest.root, 'crates/tachyon-core/src/behavior/polyglot', `${stem}_runtime.${language}`), 'utf8');
  const fixtures = await readFile(join(NativeBehaviorTest.root, 'crates/tachyon-core/tests/fixtures/behavior', `${stem}-runtime.${language}`), 'utf8');
  const code = emittedPath ? await readFile(emittedPath, 'utf8') : emitCode(language, template);
  if (language === 'cs') await checkCSharp(directory, code, fixtures, emittedPath);
  else await checkSwift(directory, code, fixtures, emittedPath);
}

async function main() {
  const directory = await mkdtemp(join(tmpdir(), 'tachyon-behavior-native-'));
  const { emittedPath, selected } = parseArguments();
  try {
    for (const language of selected) await checkLanguage(directory, language, emittedPath);
  } finally { await rm(directory, { recursive: true, force: true }); }
}

await main();
