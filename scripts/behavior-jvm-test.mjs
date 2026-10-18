import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

class JvmTest {
  static root = new URL('../', import.meta.url);
  static builtinsLiteral = /const NONE: &'static str = ("(?:[^"\\]|\\.)*");/;
}

function run(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 120000, maxBuffer: 4 * 1024 * 1024 });
  assert.equal(result.error, undefined, `${command}: ${result.error}`);
  assert.equal(result.status, 0, `${command} failed:\n${result.stdout}\n${result.stderr}`);
  if (result.stdout.trim()) console.log(result.stdout.trim());
}

async function readFixtures(extension) {
  const fixture = await readFile(new URL(`crates/tachyon-core/tests/fixtures/behavior/JvmRuntime.${extension}`, JvmTest.root), 'utf8');
  const queries = await readFile(new URL(`crates/tachyon-core/tests/fixtures/behavior/JvmQueries.${extension}`, JvmTest.root), 'utf8');
  const configuration = await readFile(new URL(`crates/tachyon-core/tests/fixtures/behavior/JvmConfiguration.${extension}`, JvmTest.root), 'utf8');
  const validation = await readFile(new URL(`crates/tachyon-core/tests/fixtures/behavior/JvmValidation.${extension}`, JvmTest.root), 'utf8');
  return { fixture, queries, configuration, validation };
}

function runtimeReplacements(language) {
  const configurations = language === 'java'
    ? 'case "RuntimeAdapters": case "SelectedAdapters": return RuntimeAdapters.configure(execution);'
    : '"RuntimeAdapters", "SelectedAdapters" -> RuntimeAdapters.configure(execution)';
  const selections = language === 'java'
    ? 'case "SelectedAdapters": return active(options, new String[]{"test", "development"}, "YON_ENABLED", "true", false);'
    : '"SelectedAdapters" -> active(options, arrayOf("test", "development"), "YON_ENABLED", "true", false)';
  const properties = language === 'java'
    ? 'case "Settings": return bind("YON_APP_", new Property[]{new Property("title", "TITLE", "string", null, false, null, null), new Property("count", "COUNT", "integer", 2L, true, 1, 100), new Property("enabled", "ENABLED", "boolean", false, true, null, null), new Property("ratio", "RATIO", "number", 1.5, true, 0, 10)}, values);'
    : '"Settings" -> bind("YON_APP_", arrayOf(Property("title", "TITLE", "string", null, false, null, null), Property("count", "COUNT", "integer", 2L, true, 1, 100), Property("enabled", "ENABLED", "boolean", false, true, null, null), Property("ratio", "RATIO", "number", 1.5, true, 0, 10)), values)';
  const validations = language === 'java'
    ? 'case "Input": return new Rule[]{new Rule("id", true, false, null, null, null, null), new Rule("name", false, true, 2L, 8L, null, null), new Rule("price", false, false, null, null, 0, 10), new Rule("optional", false, false, 0L, 2L, null, null)};'
    : '"Input" -> arrayOf(Rule("id", true, false, null, null, null, null), Rule("name", false, true, 2L, 8L, null, null), Rule("price", false, false, null, null, 0, 10), Rule("optional", false, false, 0L, 2L, null, null))';
  const valueBindings = language === 'java'
    ? 'case "ApplicationValues": return bindValues(new Property[]{new Property("port", "yon.server.port", "integer", 8080L, true, null, null), new Property("count", "yon.app.count", "integer", 5L, true, 1, 100), new Property("redis", "yon.redis.url", "string", "redis://127.0.0.1:6379", true, null, null)}, application);'
    : '"ApplicationValues" -> bindValues(arrayOf(Property("port", "yon.server.port", "integer", 8080L, true, null, null), Property("count", "yon.app.count", "integer", 5L, true, 1, 100), Property("redis", "yon.redis.url", "string", "redis://127.0.0.1:6379", true, null, null)), application)';
  const entities = language === 'java'
    ? 'case "Product": return new Entity("Product", "data", "products", "id");'
    : '"Product" -> Entity("Product", "data", "products", "id")';
  const evictions = language === 'java'
    ? 'if (type.equals("Catalog") && method.equals("list")) { Policy policy = new Policy("catalog", 30000, 65536, 128); String key = cacheKey(type, method, arguments, new int[]{0}, policy, execution, "cache:evict"); configuration.caches.get("catalog").delete(key, execution); return null; }'
    : 'if (type == "Catalog" && method == "list") { val policy = Policy("catalog", 30000, 65536, 128); val key = cacheKey(type, method, arguments, intArrayOf(0), policy, execution, "cache:evict"); configuration.caches["catalog"]!!.delete(key, execution); return@operation null }';
  return { configurations, selections, properties, validations, valueBindings, entities, evictions };
}

function fillRuntime(template, language) {
  let runtime = template;
  const { configurations, selections, properties, validations, valueBindings, entities, evictions } = runtimeReplacements(language);
  runtime = runtime.replace('__MODULE__', 'module-runtime-test').replace('__ENVIRONMENT__', 'yon')
    .replace('__CONFIGURATIONS__', configurations).replace('__ENTITIES__', entities).replace('__EVICTIONS__', evictions);
  runtime = runtime.replace('__SELECTIONS__', selections).replace('__PROPERTIES__', properties).replace('__VALIDATIONS__', validations).replace('__VALUES__', valueBindings);
  assert.doesNotMatch(runtime, /__[A-Z][A-Z_]*__/, 'Every runtime template token must be replaced');
  return runtime;
}

async function readBuiltins(language) {
  const source = await readFile(new URL(`crates/tachyon-core/src/behavior/polyglot/${language}_builtins.rs`, JvmTest.root), 'utf8');
  const literal = source.match(JvmTest.builtinsLiteral);
  assert.ok(literal, `${language}_builtins.rs must declare NONE as one string literal`);
  return JSON.parse(literal[1]);
}

async function checkKotlinNative(temporary, sources, runtime) {
  const { fixture, queries, configuration, validation } = sources;
  const nested = join(temporary, 'NativeRuntime.kt');
  const nativeFixture = `${fixture}\n${queries}\n${configuration}\n${validation}`
    .replace(/\b(YON|TAC)_/g, (_, framework) => framework === 'YON' ? 'TAC_' : 'YON_').replaceAll('"yon"', '"tac"');
  const nativeRuntime = runtime.replace('ENVIRONMENT = "yon"', 'ENVIRONMENT = "tac"')
    .replaceAll('"YON_APP_"', '"TAC_APP_"').replaceAll('"YON_ENABLED"', '"TAC_ENABLED"')
    .replaceAll('yon.', 'tac.');
  await writeFile(nested, `object NativeCompanion {\n${nativeFixture}\n${nativeRuntime}\n}\n`);
  const nativeJar = join(temporary, 'native.jar');
  run('kotlinc', [nested, '-include-runtime', '-d', nativeJar]);
  run('java', ['-cp', nativeJar, 'NativeCompanion$RuntimeChecks']);
}

async function checkLanguage(temporary, language) {
  const extension = language === 'java' ? 'java' : 'kt';
  const template = await readFile(new URL(`crates/tachyon-core/src/behavior/polyglot/${language}_runtime.${extension}`, JvmTest.root), 'utf8');
  const sources = await readFixtures(extension);
  const runtime = fillRuntime(template, language) + await readBuiltins(language);
  const { fixture, queries, configuration, validation } = sources;
  const file = join(temporary, `RuntimeChecks.${extension}`);
  await writeFile(file, `${fixture}\n${queries}\n${configuration}\n${validation}\n${runtime}`);
  if (language === 'java') {
    run('javac', ['-d', temporary, file]);
    run('java', ['-cp', temporary, 'RuntimeChecks']);
  } else {
    const jar = join(temporary, 'runtime.jar');
    run('kotlinc', [file, '-include-runtime', '-d', jar]);
    run('java', ['-cp', jar, 'RuntimeChecks']);
    await checkKotlinNative(temporary, sources, runtime);
  }
}

async function main() {
  const temporary = await mkdtemp(join(tmpdir(), 'tac-behavior-jvm-'));
  try {
    for (const language of ['java', 'kotlin']) await checkLanguage(temporary, language);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

await main();
