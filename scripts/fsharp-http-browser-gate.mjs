#!/usr/bin/env node
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'

class FsharpHttpBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(undefined, 'ordinary F#/.NET Wasm Task<BehaviorJSON> with genuine Yon, custom Setup and captured-owner cancellation; no F# DTO, cache or native async qualification')
  }
}

await FsharpHttpBrowserGate.run()
