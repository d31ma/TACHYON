import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';

class SwiftWasmSdk {
  static usage = 'Usage: node scripts/ci/install-swift-wasm-sdk.mjs <swift> <swift-sdks-folder> <download-folder>';
  static bundle = 'swift-6.3.3-RELEASE_wasm.artifactbundle';
  static platform = join('swift-6.3.3-RELEASE_wasm', 'wasm32-unknown-wasip1', 'WASI.sdk');
  static url = 'https://download.swift.org/swift-6.3.3-release/wasm-sdk/swift-6.3.3-RELEASE/swift-6.3.3-RELEASE_wasm.artifactbundle.tar.gz';
  static checksum = 'cabfa08b73bb8ac783927ecd15fa386e99d0c139c5f232445067bcf58379cae7';
  static argumentCount = 3;
  static attempts = 3;
  static pauseSeconds = 15;
  static downloadSeconds = '900';
  static installMilliseconds = 10 * 60 * 1000;

  /** @param {string[]} argumentsList */
  static run(argumentsList) {
    const [swift, sdks, downloads] = SwiftWasmSdk.#arguments(argumentsList);
    const archive = join(downloads, `${SwiftWasmSdk.bundle}.tar.gz`);
    SwiftWasmSdk.#downloadUnlessCached(archive);
    const installed = SwiftWasmSdk.#withRetries('swift sdk install', () => SwiftWasmSdk.#install(swift, sdks, archive));
    SwiftWasmSdk.#requireSuccess(installed, 'swift sdk install');
  }

  /**
   * @param {string[]} argumentsList
   * @returns {string[]}
   */
  static #arguments(argumentsList) {
    if (argumentsList.length !== SwiftWasmSdk.argumentCount) throw new Error(SwiftWasmSdk.usage);
    return argumentsList;
  }

  /** @param {string} archive */
  static #downloadUnlessCached(archive) {
    if (SwiftWasmSdk.#cachedChecksum(archive) === SwiftWasmSdk.checksum) {
      console.log(`using the cached ${archive}; its SHA-256 matches the pin`);
      return;
    }
    const downloaded = SwiftWasmSdk.#withRetries('download', () => SwiftWasmSdk.#download(archive));
    SwiftWasmSdk.#requireSuccess(downloaded, 'download');
  }

  /**
   * @param {string} archive
   * @returns {string}
   */
  static #cachedChecksum(archive) {
    return existsSync(archive) ? SwiftWasmSdk.#sha256(archive) : '';
  }

  /**
   * @param {string} step
   * @param {() => void} action
   * @returns {boolean}
   */
  static #withRetries(step, action) {
    const attempts = Array.from({ length: SwiftWasmSdk.attempts }, (_, index) => index + 1);
    return attempts.some((attempt) => SwiftWasmSdk.#tryOnce(step, attempt, action));
  }

  /**
   * @param {string} step
   * @param {number} attempt
   * @param {() => void} action
   * @returns {boolean}
   */
  static #tryOnce(step, attempt, action) {
    const pause = (attempt - 1) * SwiftWasmSdk.pauseSeconds;
    console.log(`${step}, attempt ${attempt} of ${SwiftWasmSdk.attempts}, after a ${pause} s pause`);
    execFileSync('sleep', [String(pause)]);
    try {
      action();
      return true;
    } catch (error) {
      console.log(`${step} attempt ${attempt} failed: ${SwiftWasmSdk.#message(error)}`);
      return false;
    }
  }

  /**
   * @param {boolean} succeeded
   * @param {string} step
   */
  static #requireSuccess(succeeded, step) {
    if (!succeeded) throw new Error(`${step} of the Swift SDK for WebAssembly failed ${SwiftWasmSdk.attempts} times`);
  }

  /** @param {string} archive */
  static #download(archive) {
    rmSync(archive, { force: true });
    mkdirSync(join(archive, '..'), { recursive: true });
    execFileSync('curl', [
      '--fail', '--location', '--silent', '--show-error',
      '--max-time', SwiftWasmSdk.downloadSeconds, '--retry', '3', '--retry-all-errors',
      '--output', archive, SwiftWasmSdk.url,
    ], { stdio: 'inherit' });
    SwiftWasmSdk.#requireChecksum(archive);
  }

  /** @param {string} archive */
  static #requireChecksum(archive) {
    const found = SwiftWasmSdk.#sha256(archive);
    if (found !== SwiftWasmSdk.checksum) throw new Error(`${archive} has SHA-256 ${found}, not the pinned ${SwiftWasmSdk.checksum}`);
    console.log(`${archive}: SHA-256 matches the pin`);
  }

  /**
   * @param {string} swift
   * @param {string} sdks
   * @param {string} archive
   */
  static #install(swift, sdks, archive) {
    const bundle = join(sdks, SwiftWasmSdk.bundle);
    rmSync(bundle, { recursive: true, force: true });
    execFileSync(swift, ['sdk', 'install', '--swift-sdks-path', sdks, archive], {
      stdio: 'inherit',
      timeout: SwiftWasmSdk.installMilliseconds,
      killSignal: 'SIGKILL',
    });
    SwiftWasmSdk.#requireFolder(join(bundle, SwiftWasmSdk.platform));
  }

  /** @param {string} folder */
  static #requireFolder(folder) {
    if (!existsSync(folder)) throw new Error(`${folder} is missing after swift sdk install`);
    console.log(`installed ${folder}`);
  }

  /**
   * @param {string} file
   * @returns {string}
   */
  static #sha256(file) {
    return createHash('sha256').update(readFileSync(file)).digest('hex');
  }

  /**
   * @param {unknown} error
   * @returns {string}
   */
  static #message(error) {
    return error instanceof Error ? error.message : String(error);
  }
}

SwiftWasmSdk.run(process.argv.slice(2));
