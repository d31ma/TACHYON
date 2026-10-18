use crate::Failure;
use cpp_wasm_program::CppWasmProgram;
use std::path::{Path, PathBuf};

mod cpp_wasm_program;

pub(crate) struct CppWebServices;

impl CppWebServices {
    pub(crate) const INSTANCE: &'static str = concat!(
        include_str!("browser/tac-task-answers.js"),
        include_str!("browser/tac-wasm-tasks.js"),
        r"const wasmTasks = new TacWasmTasks()
const cppResponse = await fetch(new URL('./cpp/services.wasm', import.meta.url))
const cppModule = await WebAssembly.compile(await cppResponse.arrayBuffer())
const cppNotSupported = 52
const cppLines = { 1: '', 2: '' }
const cppPrint = (descriptor, text) => {
  const lines = (cppLines[descriptor] + text).split('\n')
  cppLines[descriptor] = lines.pop()
  for (const line of lines) (descriptor === 2 ? console.error : console.log)(line)
}
const cppSystem = {
  fd_write: (descriptor, vectors, count, written) => {
    const view = new DataView(exports.memory.buffer)
    let total = 0
    for (let index = 0; index < count; index += 1) {
      const pointer = view.getUint32(vectors + index * 8, true)
      const length = view.getUint32(vectors + index * 8 + 4, true)
      cppPrint(descriptor, wasmRead(pointer, length))
      total += length
    }
    view.setUint32(written, total, true)
    return 0
  },
  proc_exit: (code) => { throw new Error(`C++ services stopped with exit code ${code}.`) },
}
for (const { module, name, kind } of WebAssembly.Module.imports(cppModule)) {
  if (module === 'wasi_snapshot_preview1' && kind === 'function') cppSystem[name] ??= () => cppNotSupported
}
const cppImports = { tachyon: { ...tachyonImports, ...wasmTasks.imports(wasmRead, (owner, text) => socketHost(owner, text)) }, wasi_snapshot_preview1: cppSystem, env: { emscripten_notify_memory_growth: () => {} } }
const cppInstance = await WebAssembly.instantiate(cppModule, cppImports)
const exports = cppInstance.exports
exports._initialize?.()
wasmTasks.attach(exports)
"
    );

    pub(crate) async fn build(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        let program = crate::tac_cpp::CppProgram::read(snapshot_root, services)?.web_source();
        let built = CppWasmProgram::compile(&program, profile).await?;
        let note = CppWasmProgram::note(profile);
        Ok((built.published(), note))
    }
}
