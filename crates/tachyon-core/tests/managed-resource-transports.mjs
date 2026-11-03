import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

class TransportTest {
  static preludes = fileURLToPath(new URL('../src/handler/preludes/', import.meta.url));
  static token = 'a'.repeat(64);
  static failure = 'Yon managed resource operation failed.';
  static payload = JSON.stringify({ method: 'GET' });
  static privateValue = 'test-only-private-value';
  static gatewayPayload = JSON.stringify({ method: 'POST', deadline_ms: 10_000 });
  static expiredGatewayPayload = JSON.stringify({ method: 'POST', deadline_ms: 0 });
  static gatewayExtensions = ['java', 'kt', 'cs', 'go'];
  static redisPayload = JSON.stringify({ method: 'PUT' });
  static goEnvironment = { GO111MODULE: 'on', GOFLAGS: '', GOPROXY: 'off', GOTOOLCHAIN: 'local', GOWORK: 'off', CGO_ENABLED: '0' };
  static goHandler = `package main

type Handler struct{}

func (Handler) GET(request *yonRequest) any {
	rows, err := DataSource{}.Query("SELECT ?", []any{"test-only-private-value"})
	if err != nil {
		return map[string]any{"error": err.Error()}
	}
	return rows
}

func (Handler) POST(request *yonRequest) *yonResponse {
	return yonJSON(YonGateway.Call("ReportClient.summarize", \`["plain",[1,2],null]\`))
}

func (Handler) PUT(request *yonRequest) (any, error) {
	return RedisTemplate{}.Set("greeting", map[string]any{"works": true}, 30)
}

func yonDispatch(request *yonRequest) *yonResponse {
	switch request.Method() {
	case "GET":
		return yonAnswer(new(Handler).GET(request))
	case "POST":
		return yonAnswer(new(Handler).POST(request))
	case "PUT":
		return yonAnswer(new(Handler).PUT(request))
	}
	return yonEmpty(405)
}
`;
  static answer(body) { return `HTTP/1.1 200 OK\r\nContent-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`; }
  static rejectedResponses = [
    'HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 2\r\n\r\n{}',
    'HTTP/1.1 403 Forbidden\r\nContent-Length: 2\r\n\r\n{}',
    'HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}',
    'HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\n\r\n',
    'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n',
    'HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\nprivate',
    'HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\n{}',
    'HTTP/1.1 200 OK\r\n\r\n{}',
    Buffer.concat([Buffer.from('HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n'), Buffer.from([255])]),
  ];
}

async function run(command, args, { cwd, endpoint = '', timeout = 120_000, input = '', environment = {} } = {}) {
  return await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd, detached: process.platform !== 'win32', stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, ...environment, YON_RESOURCE_ENDPOINT: endpoint, YON_RESOURCE_TOKEN: TransportTest.token, DOTNET_CLI_DO_NOT_USE_MSBUILD_SERVER: '1' },
    });
    const output = [[], []];
    const size = [0, 0];
    let exceeded = false;
    const stop = () => { try { process.platform === 'win32' ? child.kill('SIGKILL') : process.kill(-child.pid, 'SIGKILL'); } catch {} };
    const timer = setTimeout(() => { exceeded = true; stop(); }, timeout);
    for (const [index, stream] of [child.stdout, child.stderr].entries()) stream.on('data', bytes => {
      size[index] += bytes.length;
      if (size[index] > 2 * 1024 * 1024) { exceeded = true; stop(); } else output[index].push(bytes);
    });
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.on('close', code => {
      clearTimeout(timer);
      if (exceeded || code !== 0) reject(new Error(`${command} failed (${exceeded ? 'bounded timeout/output limit' : code}): ${Buffer.concat(output[1]).toString().slice(0, 4096)}`));
      else resolve(Buffer.concat(output[0]).toString());
    });
    child.stdin.on('error', () => {});
    child.stdin.end(input);
  });
}

async function prepareGo(root) {
  const directory = path.join(root, 'go');
  await mkdir(path.join(directory, 'yoninvocation'), { recursive: true });
  await writeFile(path.join(directory, 'go.mod'), 'module yonhandler\n\ngo 1.27\n');
  await writeFile(path.join(directory, 'yon.go'), await readFile(path.join(TransportTest.preludes, 'yon.go'), 'utf8'));
  await writeFile(path.join(directory, 'yoninvocation', 'invocation.go'), await readFile(path.join(TransportTest.preludes, 'yon_invocation.go'), 'utf8'));
  await writeFile(path.join(directory, 'handler.go'), TransportTest.goHandler);
  const program = path.join(directory, process.platform === 'win32' ? 'program.exe' : 'program');
  await run('go', ['build', '-o', program, '.'], { cwd: directory, environment: TransportTest.goEnvironment });
  return [program, []];
}

async function prepare(root, extension) {
  if (extension === 'go') return await prepareGo(root);
  let prelude = (await readFile(path.join(TransportTest.preludes, `yon.${extension}`), 'utf8')).replaceAll('__YON_HANDLER__', 'Handler');
  const handlers = {
    java: `class Handler { static Object GET(__YonFrame request) { try { return new DataSource().query("SELECT ?", java.util.List.of("${TransportTest.privateValue}")); } catch (Exception error) { return java.util.Map.of("error", error.getMessage()); } } static Object POST(__YonFrame request) { return YonGateway.call("ReportClient.summarize", new Object[]{"plain", java.util.List.of(1, 2), null}); } }`,
    kt: `object Handler { @JvmStatic fun GET(request: __YonFrame): Any? = try { DataSource().query("SELECT ?", listOf("${TransportTest.privateValue}")) } catch (error: Exception) { mapOf("error" to error.message) } @JvmStatic fun POST(request: __YonFrame): Any? = YonGateway.call("ReportClient.summarize", arrayOf<Any?>("plain", listOf(1, 2), null)) }`,
    cs: `class Handler { public static object GET(__YonFrame request) { try { return new DataSource().Query("SELECT ?", new object[] { "${TransportTest.privateValue}" }); } catch (System.Exception error) { return new { error = error.Message }; } } public static object POST(__YonFrame request) => __YonAnswer.Json(YonGateway.Call("ReportClient.summarize", "[\\"plain\\",[1,2],null]")); }`,
    php: `final class Handler { public static function GET(): mixed { try { return (new DataSource())->query('SELECT ?', ['${TransportTest.privateValue}']); } catch (Throwable $error) { return ['error' => $error->getMessage()]; } } }`,
  };
  const directory = path.join(root, extension);
  await mkdir(directory);
  const source = path.join(directory, `Program.${extension}`);
  const entry = extension === 'php' ? '\nYonMain::run();\n' : '\n';
  await writeFile(source, prelude + '\n' + handlers[extension] + entry);
  if (extension === 'java') {
    await run('javac', ['-d', directory, source], { cwd: directory });
    return ['java', ['-cp', directory, 'YonLauncher']];
  }
  if (extension === 'kt') {
    const jar = path.join(directory, 'program.jar');
    await run('kotlinc', [source, '-include-runtime', '-d', jar], { cwd: directory });
    return ['java', ['-jar', jar]];
  }
  if (extension === 'cs') {
    const project = path.join(directory, 'Transport.csproj');
    const sdk = (await run('dotnet', ['--version'], { cwd: directory })).trim();
    const major = sdk.match(/^(\d+)\./)?.[1];
    assert.ok(major && Number(major) >= 9, 'Managed transport contracts require .NET SDK 9 or newer');
    await writeFile(project, `<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net${major}.0</TargetFramework><Nullable>disable</Nullable><EnableDefaultCompileItems>true</EnableDefaultCompileItems><UseSharedCompilation>false</UseSharedCompilation></PropertyGroup></Project>`);
    const output = path.join(directory, 'out');
    await run('dotnet', ['build', project, '--disable-build-servers', '--nologo', '-o', output], { cwd: directory });
    return ['dotnet', [path.join(output, 'Transport.dll')]];
  }
  return ['php-cgi', ['-C', '-d', 'display_errors=0', '-d', 'expose_php=0', '-d', 'output_buffering=0', '-f', source]];
}

async function peer(response, invoke) {
  const requests = [];
  const sockets = new Set();
  const server = net.createServer(socket => {
    sockets.add(socket); socket.on('close', () => sockets.delete(socket)); socket.on('error', () => {});
    let bytes = Buffer.alloc(0);
    socket.on('data', chunk => {
      bytes = Buffer.concat([bytes, chunk]);
      assert.ok(bytes.length < 1024 * 1024 + 16384);
      const end = bytes.indexOf('\r\n\r\n');
      if (end < 0) return;
      const headers = bytes.subarray(0, end).toString('ascii');
      const length = Number(headers.match(/content-length: (\d+)/i)?.[1]);
      if (bytes.length < end + 4 + length) return;
      requests.push({ headers, body: JSON.parse(bytes.subarray(end + 4, end + 4 + length)) });
      socket.removeAllListeners('data');
      if (typeof response === 'function') response(socket);
      else socket.end(response);
    });
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  try { return await invoke(`http://127.0.0.1:${server.address().port}/`, requests); }
  finally { for (const socket of sockets) socket.destroy(); await new Promise(resolve => server.close(resolve)); }
}

async function checkValidResponse(invoke, validBody) {
  await peer(`HTTP/1.1 200 OK\r\nContent-Length: ${validBody.length}\r\n\r\n${validBody}`, async (endpoint, requests) => {
    assert.deepEqual(await invoke(endpoint), [{ value: 'ok' }]);
    assert.equal(requests.length, 1);
    assert.match(requests[0].headers, new RegExp(`Authorization: Bearer ${TransportTest.token}`));
    assert.deepEqual(requests[0].body, { operation: 'sql', sql: 'SELECT ?', parameters: [TransportTest.privateValue], mode: 'rows' });
  });
  return 4;
}

async function checkRejectedEndpoints(invoke) {
  let assertions = 0;
  for (const endpoint of ['', 'http://localhost:1234/', 'http://127.0.0.1:65536/', 'https://127.0.0.1:1234/', 'http://127.0.0.1:1234/private']) {
    assert.deepEqual(await invoke(endpoint), { error: TransportTest.failure }); assertions++;
  }
  return assertions;
}

async function checkRedirectNotFollowed(invoke, validBody) {
  await peer(`HTTP/1.1 200 OK\r\nContent-Length: ${validBody.length}\r\n\r\n${validBody}`, async (redirectTarget, followed) => {
    await peer(`HTTP/1.1 302 Found\r\nLocation: ${redirectTarget}\r\nContent-Length: 2\r\n\r\n{}`, async endpoint => {
      assert.deepEqual(await invoke(endpoint), { error: TransportTest.failure });
    });
    assert.equal(followed.length, 0, 'Resource clients must never follow a redirect');
  });
  return 2;
}

async function checkStalledBody(invoke) {
  await peer(socket => socket.write('HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n'), async endpoint => {
    const started = Date.now();
    assert.deepEqual(await invoke(endpoint), { error: TransportTest.failure });
    assert.ok(Date.now() - started < 8500, 'A stalled body must respect the five-second transport deadline');
  });
  return 2;
}

async function checkRejectedResponses(invoke) {
  let assertions = 0;
  for (const response of TransportTest.rejectedResponses) {
    await peer(response, async endpoint => { assert.deepEqual(await invoke(endpoint), { error: TransportTest.failure }); assertions++; });
  }
  return assertions;
}

async function checkGatewayAnswers(invoke) {
  await peer(TransportTest.answer('{"result":{"value":"ok"}}'), async (endpoint, requests) => {
    assert.deepEqual(await invoke(endpoint), { result: { value: 'ok' } });
    assert.equal(requests.length, 1);
    assert.match(requests[0].headers, new RegExp(`Authorization: Bearer ${TransportTest.token}`));
    assert.deepEqual(requests[0].body, { operation: 'gateway', key: 'ReportClient.summarize', arguments: ['plain', [1, 2], null] });
  });
  await peer(TransportTest.answer('{"error":"capacity"}'), async endpoint => {
    assert.deepEqual(await invoke(endpoint), { error: 'capacity' });
  });
  return 5;
}

async function checkGatewayFailures(invoke) {
  let assertions = 0;
  for (const endpoint of ['', 'http://localhost:1234/', 'http://127.0.0.1:1234/private']) {
    assert.deepEqual(await invoke(endpoint), { error: 'adapter' }); assertions++;
  }
  for (const response of [...TransportTest.rejectedResponses, TransportTest.answer('{"other":1}')]) {
    await peer(response, async endpoint => { assert.deepEqual(await invoke(endpoint), { error: 'adapter' }); assertions++; });
  }
  return assertions;
}

async function checkGatewayWithoutTimeLeft(invokeWith) {
  await peer(TransportTest.answer('{"result":1}'), async (endpoint, requests) => {
    assert.deepEqual(await invokeWith(endpoint, TransportTest.expiredGatewayPayload), { error: 'timeout' });
    assert.equal(requests.length, 0, 'A gateway call without time left must not reach the endpoint');
  });
  return 2;
}

async function checkGateway(invokeWith) {
  const invoke = endpoint => invokeWith(endpoint, TransportTest.gatewayPayload);
  let assertions = 0;
  assertions += await checkGatewayAnswers(invoke);
  assertions += await checkGatewayFailures(invoke);
  assertions += await checkGatewayWithoutTimeLeft(invokeWith);
  return assertions;
}

async function checkRedisRequest(invokeWith) {
  await peer(TransportTest.answer('true'), async (endpoint, requests) => {
    assert.equal(await invokeWith(endpoint, TransportTest.redisPayload), true);
    assert.equal(requests.length, 1);
    assert.deepEqual(requests[0].body, { operation: 'redis', command: 'set', key: 'greeting', value: { works: true }, ttlSeconds: 30 });
  });
  return 3;
}

async function checkRequestHead(invoke) {
  await peer(TransportTest.answer('[]'), async (endpoint, requests) => {
    await invoke(endpoint);
    const port = new URL(endpoint).port;
    assert.match(requests[0].headers, /^POST \/ HTTP\/1\.1\r\n/);
    assert.match(requests[0].headers, new RegExp(`\\r\\nHost: 127\\.0\\.0\\.1:${port}(\\r\\n|$)`));
    assert.match(requests[0].headers, /\r\nContent-Type: application\/json(\r\n|$)/);
    assert.doesNotMatch(requests[0].headers, /\r\n(origin|transfer-encoding):/i);
  });
  return 4;
}

async function checkLongHead(invoke) {
  const padding = `X-Padding: ${'x'.repeat(8192)}\r\n`;
  await peer(`HTTP/1.1 200 OK\r\n${padding}Content-Length: 2\r\n\r\n[]`, async endpoint => {
    assert.deepEqual(await invoke(endpoint), { error: TransportTest.failure });
  });
  return 1;
}

async function checkExtension(root, extension) {
  const [program, args] = await prepare(root, extension);
  const invokeWith = async (endpoint, input) => {
    const envelope = JSON.parse(await run(program, args, { cwd: root, endpoint, input, timeout: 15_000 }));
    return JSON.parse(envelope.body);
  };
  const invoke = endpoint => invokeWith(endpoint, TransportTest.payload);
  const validBody = '[{"value":"ok"}]';
  let assertions = 0;
  assertions += await checkValidResponse(invoke, validBody);
  assertions += await checkRejectedEndpoints(invoke);
  assertions += await checkRedirectNotFollowed(invoke, validBody);
  assertions += await checkStalledBody(invoke);
  assertions += await checkRejectedResponses(invoke);
  if (TransportTest.gatewayExtensions.includes(extension)) assertions += await checkGateway(invokeWith);
  if (extension === 'go') assertions += await checkRedisRequest(invokeWith) + await checkRequestHead(invoke) + await checkLongHead(invoke);
  console.log(`PASS ${extension} managed-resource transport`);
  return assertions;
}

async function main() {
  const root = await mkdtemp(path.join(os.tmpdir(), 'yon-managed-transport-'));
  let assertions = 0;
  try {
    for (const extension of ['java', 'kt', 'cs', 'php', 'go']) assertions += await checkExtension(root, extension);
    console.log(`PASS ${assertions} direct transport assertions`);
  } finally { await rm(root, { recursive: true, force: true }); }
}

await main();
