import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

class LuaRuntimeBuild {
  static usage = 'Usage: node scripts/ci/build-lua-runtime.mjs <ty>';
  static files = {
    'client/pages/tac.html': '<lua-count />\n',
    'client/components/lua/count/tac.html': '<p id="lua-count">{luaWarmup.count}</p>\n',
    'client/services/LuaWarmupService.lua': `-- @Service
---@class LuaWarmupService
LuaWarmupService = {
  ---@type integer
  count = 0,
}
`,
  };

  /** @param {string[]} argumentsList */
  static run(argumentsList) {
    const binary = LuaRuntimeBuild.#binary(argumentsList);
    const project = mkdtempSync(join(tmpdir(), 'ty-lua-runtime-'));
    LuaRuntimeBuild.#buildThenRemove(binary, project);
  }

  /**
   * @param {string[]} argumentsList
   * @returns {string}
   */
  static #binary(argumentsList) {
    if (argumentsList.length !== 1) throw new Error(LuaRuntimeBuild.usage);
    return resolve(argumentsList[0]);
  }

  /**
   * @param {string} binary
   * @param {string} project
   */
  static #buildThenRemove(binary, project) {
    try {
      LuaRuntimeBuild.#write(project);
      LuaRuntimeBuild.#build(binary, project);
    } finally {
      rmSync(project, { recursive: true, force: true });
    }
  }

  /** @param {string} project */
  static #write(project) {
    for (const [name, contents] of Object.entries(LuaRuntimeBuild.files)) {
      mkdirSync(dirname(join(project, name)), { recursive: true });
      writeFileSync(join(project, name), contents);
    }
  }

  /**
   * @param {string} binary
   * @param {string} project
   */
  static #build(binary, project) {
    const started = Date.now();
    execFileSync(binary, ['build', project], { stdio: 'inherit' });
    const seconds = ((Date.now() - started) / 1000).toFixed(1);
    console.log(`A Lua page built in ${seconds} s; the Tac Lua runtime is now in the Tachyon cache`);
  }
}

LuaRuntimeBuild.run(process.argv.slice(2));
