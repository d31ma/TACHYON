#!/usr/bin/env node
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

class CompanionTest {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty');
  static target = { darwin: 'macos', linux: 'linux', win32: 'windows' }[process.platform];
}

assert.ok(CompanionTest.target, 'native companion gate requires a desktop host');

function write(project, name, source) {
  const file = path.join(project, name);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, source);
}

const run = (command, args) => {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 240000, maxBuffer: 2 * 1024 * 1024 });
  assert.equal(result.status, 0, `${command} failed: ${result.error || result.stderr || result.stdout}`);
  return result.stdout;
};

class NativeServices {
  static first = `#[derive(Default)]
struct NativeFirstService {
    count: i64,
    application_event_publisher: ApplicationEventPublisher,
}
impl NativeFirstService {
    const RECEIVED: &'static str = "received";

    fn doubled(&self) -> i64 { self.count * 2 }
    fn process_id(&self) -> i64 { std::process::id() as i64 }
    fn announce(&self) -> String {
        self.application_event_publisher.publish_event(crate::models::NativeEvent { text: Self::RECEIVED.to_owned() }, false);
        Self::RECEIVED.to_owned()
    }
}
`
  static second = `#[derive(Default)]
struct NativeSecondService {
    count: i64,
}
impl NativeSecondService {
    fn doubled(&self) -> i64 { self.count * 2 }
}
`
}

function writeServices(project) {
  write(project, 'client/manifest.json', JSON.stringify({ name: 'CompanionGate', start_url: '/', tac: { application: { id: 'dev.tachyon.companion-gate', version: '0.0.1' } } }));
  write(project, 'client/models/NativeEvent.rs', 'pub struct NativeEvent {\n    pub text: String,\n}\n');
  for (const [route, name, source] of [['', 'First', NativeServices.first], ['second/', 'Second', NativeServices.second]]) {
    const instance = `native${name}`;
    write(project, `client/pages/${route}tac.html`, `<native-${name.toLowerCase()} />`);
    write(project, `client/components/native/${name.toLowerCase()}/tac.html`, `<main><p>{${instance}.count}</p></main>`);
    write(project, `client/services/Native${name}Service.rs`, source);
  }
}

function buildLibrary(project) {
  run(CompanionTest.binary, ['bundle', project, '--native', '--target', CompanionTest.target]);
  const staged = path.join(project, 'dist', CompanionTest.target, 'project', 'companion.rs');
  assert.ok(existsSync(staged), 'ty must emit a native companion compilation unit');
  const suffix = process.platform === 'win32' ? '.dll' : process.platform === 'darwin' ? '.dylib' : '.so';
  const library = path.join(project, `probe${suffix}`);
  run('rustc', ['--edition', '2024', '--crate-type', 'cdylib', '-O', staged, '-o', library]);
  return library;
}

function abiProbe() {
  return String.raw`
import ctypes, json, os, sys
library = ctypes.CDLL(sys.argv[1])
library.tac_native_invoke.argtypes = [ctypes.c_char_p]
library.tac_native_invoke.restype = ctypes.c_void_p
library.tac_native_free.argtypes = [ctypes.c_void_p]
def invoke_raw(raw):
    answer = library.tac_native_invoke(raw)
    assert answer, 'native ABI returned a null response'
    try:
        return json.loads(ctypes.string_at(answer).decode())
    finally:
        library.tac_native_free(answer)
owners = {'nativeFirst': '00000000-0000-4000-8000-000000000001', 'nativeSecond': '00000000-0000-4000-8000-000000000002', 'nativeMissing': '00000000-0000-4000-8000-000000000003'}
def invoke(service, op, **values):
    return invoke_raw(json.dumps(dict(route='/', service=service, owner=owners[service], op=op, **values)).encode())
for malformed in [b'{', b'[]', b'x' * 65537]:
    assert 'error' in invoke_raw(malformed), 'malformed/oversized native request accepted'
members = invoke('nativeFirst', 'init')['value']
assert 'count' in members['fields'] and 'doubled' in members['methods'], members
assert invoke('nativeFirst', 'get', name='count')['value'] == 0
invoke('nativeFirst', 'set', name='count', value=7)
assert invoke('nativeFirst', 'get', name='count')['value'] == 7
assert invoke('nativeFirst', 'call', name='doubled', args=[])['value'] == 14
assert 'error' in invoke('nativeSecond', 'get', name='count'), 'uninitialized owner was accepted'
invoke('nativeSecond', 'init')
assert invoke('nativeSecond', 'get', name='count')['value'] == 0, 'service state leaked'
invoke('nativeSecond', 'set', name='count', value=19)
assert invoke('nativeSecond', 'call', name='doubled', args=[])['value'] == 38
assert invoke('nativeFirst', 'get', name='count')['value'] == 7, 'a second activation retired the first'
invoke('nativeFirst', 'dispose')
assert 'error' in invoke('nativeFirst', 'get', name='count'), 'disposed instance owner was accepted'
owners['nativeFirst'] = '00000000-0000-4000-8000-000000000004'
invoke('nativeFirst', 'init')
assert invoke('nativeFirst', 'get', name='count')['value'] == 0, 'new instance owner retained stale state'
assert invoke('nativeSecond', 'get', name='count')['value'] == 19, 'sibling activation was disturbed'
assert invoke('nativeFirst', 'call', name='processId', args=[])['value'] == os.getpid(), 'not target-native execution'
assert 'error' in invoke('nativeFirst', 'call', name='missing', args=[])
assert 'error' in invoke('nativeFirst', 'unsupported-operation')
assert 'error' in invoke('nativeMissing', 'call', name='doubled', args=[])
assert invoke('nativeMissing', 'init').get('code') == 'TY_NATIVE_SERVICE', 'an unknown service must be refused, not answered with nothing'
owners['nativeFirst'] = '00000000-0000-4000-8000-000000000005'
invoke('nativeFirst', 'init')
received = []
callback_type = ctypes.CFUNCTYPE(None, ctypes.c_char_p)
@callback_type
def receive(raw):
    received.append(json.loads(raw.decode()))
library.tac_native_set_emit.argtypes = [callback_type]
library.tac_native_set_emit(receive)
invoke('nativeFirst', 'call', name='announce', args=[])
assert received == [{'route': '/', 'service': 'nativeFirst', 'owner': owners['nativeFirst'], 'name': 'NativeEvent', 'value': {'text': 'received'}, 'retain': False}], received
print('Native service ABI passed: field read/write, method call, per-instance isolation and coexistence, OS access, publish callback, unknown-service rejection.')
`;
}

function main() {
  const project = mkdtempSync(path.join(tmpdir(), 'ty-native-companion-'));
  try {
    writeServices(project);
    const library = buildLibrary(project);
    console.log(run(process.platform === 'win32' ? 'python' : 'python3', ['-c', abiProbe(), library]).trim());
  } finally {
    rmSync(project, { recursive: true, force: true });
  }
}

main();
