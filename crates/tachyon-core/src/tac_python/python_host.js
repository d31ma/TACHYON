class TacPythonHost {
  static async start(program) {
    const root = new URL('./python/314.0.7/', import.meta.url)
    const loader = await import(new URL('pyodide.mjs', root).href)
    const runtime = await loader.loadPyodide({ indexURL: root.href, lockFileURL: new URL('pyodide-lock.json', root).href, stdLibURL: new URL('python_stdlib.zip', root).href })
    runtime.runPython(program)
    const type = runtime.globals.get('_TacPythonRuntime')
    const invoke = type.invoke
    const shutdown = type.shutdown
    type.destroy()
    return new TacPythonHost(invoke, shutdown)
  }

  constructor(invoke, shutdown) {
    this.invoke = invoke
    this.shutdown = shutdown
    this.disposed = false
    addEventListener('pagehide', (event) => this.pageHidden(event))
  }

  exchange(text) {
    this.requireActive()
    return this.invoke(text)
  }

  pageHidden(event) {
    this.releaseForPage(event.persisted)
  }

  requireActive() {
    if (this.disposed) throw new Error('Python Tac host is disposed.')
  }

  releaseForPage(persisted) {
    if (!persisted && !this.disposed) this.dispose()
  }

  dispose() {
    this.release()
  }

  release() {
    this.disposed = true
    try {
      this.shutdown()
    } finally {
      this.invoke.destroy()
      this.shutdown.destroy()
    }
  }
}

const pythonHost = await TacPythonHost.start(__TAC_PYTHON_PROGRAM__)
const transport = (text) => pythonHost.exchange(text)
