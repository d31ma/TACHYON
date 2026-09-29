import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

export class ProbeFixture {
  static scriptBudgetKilobytes = 320;
  static shortLanguageNames = ['js', 'cs', 'fs', 'kt'];
  static buildTimeoutMilliseconds = 900_000;
  static outputLimitBytes = 16 * 1024 * 1024;

  /**
   * @param {string} project
   * @param {Array<{key: string, label: string, service: string, file: string, source: string}>} runtimes
   * @returns {void}
   */
  static write(project, runtimes) {
    ProbeFixture._write(project, 'client/manifest.json', ProbeFixture._manifest());
    ProbeFixture._write(project, 'client/pages/tac.html', '<probe-home />\n');
    ProbeFixture._write(project, 'client/components/probe/home/tac.html', '<main aria-label="Probes"><p>Probes</p></main>\n');
    ProbeFixture._writeRuntimes(project, runtimes);
  }

  /**
   * @param {string} binary
   * @param {string} project
   * @returns {void}
   */
  static build(binary, project) {
    const built = spawnSync(binary, ['build', project], {
      encoding: 'utf8',
      timeout: ProbeFixture.buildTimeoutMilliseconds,
      killSignal: 'SIGKILL',
      maxBuffer: ProbeFixture.outputLimitBytes,
    });
    ProbeFixture._requireSuccess(built);
  }

  /**
   * @param {string} project
   * @returns {string}
   */
  static output(project) {
    return path.join(project, 'dist');
  }

  /**
   * @param {string} project
   * @param {Array<{key: string, label: string, service: string, file: string, source: string}>} runtimes
   * @returns {void}
   */
  static _writeRuntimes(project, runtimes) {
    for (const runtime of runtimes) {
      ProbeFixture._writeRuntime(project, runtime);
    }
  }

  /**
   * @param {string} project
   * @param {{key: string, label: string, service: string, file: string, source: string}} runtime
   * @returns {void}
   */
  static _writeRuntime(project, runtime) {
    ProbeFixture._write(project, runtime.file, runtime.source);
    ProbeFixture._write(project, `client/pages/${runtime.key}/tac.html`, `<probe-${runtime.key} />\n`);
    ProbeFixture._write(
      project,
      `client/components/probe/${runtime.key}/tac.html`,
      `<main aria-label="${runtime.label} probe">\n  <p id="count">{${runtime.service}.count}</p>\n</main>\n`,
    );
  }

  /** @returns {string} */
  static _manifest() {
    return JSON.stringify({
      tac: {
        budgets: { script: ProbeFixture.scriptBudgetKilobytes },
        names: { words: ProbeFixture.shortLanguageNames },
        visibility: { check: false },
      },
    });
  }

  /**
   * @param {import('node:child_process').SpawnSyncReturns<string>} built
   * @returns {void}
   */
  static _requireSuccess(built) {
    if (built.status === 0) return;
    throw new Error(`the probe project did not build: ${built.error?.message || built.stderr || built.stdout}`);
  }

  /**
   * @param {string} project
   * @param {string} relative
   * @param {string} contents
   * @returns {void}
   */
  static _write(project, relative, contents) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, contents);
  }
}
