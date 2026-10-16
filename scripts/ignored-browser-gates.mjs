#!/usr/bin/env node
import { execFileSync, spawn } from 'node:child_process'
import { mkdtempSync, realpathSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from './cargo-artifacts.mjs'

/** @typedef {{name:string,group:string,prefix:string,tests:string[]}} GateFamily */
/** @typedef {{test:string,passed:boolean,seconds:number}} GateResult */

class IgnoredBrowserGates {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  static groups = ['dotnet', 'kotlin', 'java']
  static deadlineMilliseconds = 20 * 60 * 1000
  static millisecondsPerSecond = 1000
  static module = 'tac_web'

  /** @type {GateFamily[]} */
  static families = [
    {
      name: 'dotnet-http',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: ['dotnet_http_browser_fixture::dotnet_default_http_uses_real_yon_and_captured_browser_owners']
    },
    {
      name: 'dotnet-http-cache',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: [
        'dotnet_http_cache_browser_fixture::dotnet_cached_http_client_uses_public_yon_and_revalidates_after_clear',
        'dotnet_http_cache_browser_fixture::dotnet_cached_http_client_existing_build_uses_public_yon_and_child_cancellation',
        'dotnet_http_cache_browser_fixture::dotnet_cached_http_client_secure_origin_certificate_is_explicit',
        'dotnet_http_cache_browser_fixture::dotnet_cached_http_client_uses_real_service_worker_cache_and_revalidation'
      ]
    },
    {
      name: 'dotnet-task',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_TASK',
      tests: ['dotnet_task_browser_fixture::dotnet_tasks_use_the_ordinary_compiler_and_real_browser_continuations']
    },
    {
      name: 'dotnet-task-model',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: [
        'dotnet_task_model_browser_fixture::dotnet_registered_task_models_use_real_yon_and_owned_client_disposal',
        'dotnet_task_model_browser_fixture::dotnet_registered_task_models_existing_build_uses_real_yon_and_owned_disposal'
      ]
    },
    {
      name: 'dotnet-task-collection',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: [
        'dotnet_task_collection_browser_fixture::dotnet_registered_task_collection_fields_use_real_yon_and_owned_disposal',
        'dotnet_task_collection_browser_fixture::dotnet_registered_task_collection_existing_build_uses_real_yon_and_owned_disposal'
      ]
    },
    {
      name: 'first-app-csharp',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: [
        'first_app_csharp_browser_fixture::first_app_csharp_cumulative_step_four_uses_published_yon',
        'first_app_csharp_browser_fixture::first_app_csharp_existing_build_uses_published_yon'
      ]
    },
    {
      name: 'first-app-csharp-tests',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: [
        'first_app_csharp_browser_fixture::first_app_csharp_cumulative_step_five_uses_owned_typed_fake',
        'first_app_csharp_browser_fixture::first_app_csharp_post_step_five_browser_uses_published_yon'
      ]
    },
    {
      name: 'first-app-fsharp',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: ['first_app_csharp_browser_fixture::first_app_fsharp_cumulative_step_four_uses_published_yon']
    },
    {
      name: 'first-app-fsharp-tests',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: [
        'first_app_csharp_browser_fixture::first_app_fsharp_cumulative_step_five_uses_owned_typed_fake',
        'first_app_csharp_browser_fixture::first_app_fsharp_post_step_five_browser_uses_published_yon'
      ]
    },
    {
      name: 'fsharp-http',
      group: 'dotnet',
      prefix: 'TAC_DOTNET_HTTP',
      tests: ['fsharp_http_browser_fixture::fsharp_browser_http_uses_existing_task_host_and_real_yon']
    },
    {
      name: 'document-title',
      group: 'kotlin',
      prefix: 'TAC_DOCUMENT_TITLE',
      tests: ['document_title_browser_fixture::document_title_uses_the_ordinary_kotlin_compiler_and_real_browser_activation']
    },
    {
      name: 'preference-strings',
      group: 'java',
      prefix: 'TAC_PREFERENCE',
      tests: ['preference_strings_browser_fixture::java_preference_strings_use_the_ordinary_compiler_and_real_browser_activation']
    }
  ]

  /** @returns {string[]} */
  static chosenGroups() {
    const option = process.argv.find(argument => argument.startsWith('--groups='))
    if (!option) return IgnoredBrowserGates.groups
    return option.slice('--groups='.length).split(',').filter(group => group !== '')
  }

  /** @returns {string} */
  static sourceCommit() {
    return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: IgnoredBrowserGates.repo, encoding: 'utf8' }).trim()
  }

  /**
   * @param {GateFamily} family
   * @param {string} binary
   * @param {string} commit
   * @returns {NodeJS.ProcessEnv}
   */
  static environment(family, binary, commit) {
    const root = realpathSync(mkdtempSync(path.join(tmpdir(), `tachyon-${family.name}-`)))
    return {
      ...process.env,
      TAC_BIN: binary,
      TAC_CACHE_DIR: path.join(root, 'cache'),
      [`${family.prefix}_FIXTURE_OUT`]: root,
      [`${family.prefix}_SOURCE_COMMIT`]: commit
    }
  }

  /**
   * @param {string} test
   * @param {NodeJS.ProcessEnv} environment
   * @returns {Promise<GateResult>}
   */
  static run(test, environment) {
    const name = `${IgnoredBrowserGates.module}::${test}`
    const arguments_ = ['test', '--locked', '-p', 'tachyon-core', '--lib', name, '--', '--ignored', '--exact']
    const started = Date.now()
    return new Promise(resolve => {
      const child = spawn('cargo', arguments_, { cwd: IgnoredBrowserGates.repo, env: environment, stdio: 'inherit', detached: true })
      const deadline = setTimeout(() => IgnoredBrowserGates.stopGroup(child.pid), IgnoredBrowserGates.deadlineMilliseconds)
      child.once('close', code => {
        clearTimeout(deadline)
        const seconds = Math.round((Date.now() - started) / IgnoredBrowserGates.millisecondsPerSecond)
        resolve({ test: name, passed: code === 0, seconds })
      })
    })
  }

  /** @param {number|undefined} leader */
  static stopGroup(leader) {
    if (leader === undefined) return
    try {
      process.kill(-leader, 'SIGKILL')
    } catch (error) {
      console.error(`could not stop process group ${leader}: ${error}`)
    }
  }

  static async main() {
    const groups = IgnoredBrowserGates.chosenGroups()
    const unknown = groups.filter(group => !IgnoredBrowserGates.groups.includes(group))
    const families = IgnoredBrowserGates.families.filter(family => groups.includes(family.group))
    if (unknown.length > 0 || families.length === 0) {
      console.error(`ignored browser gates: unknown group ${unknown.join(' ') || 'none chosen'}; nothing was run`)
      process.exitCode = 1
      return
    }
    const binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
    const commit = IgnoredBrowserGates.sourceCommit()
    /** @type {GateResult[]} */
    const results = []
    for (const family of families) {
      const environment = IgnoredBrowserGates.environment(family, binary, commit)
      for (const test of family.tests) {
        const result = await IgnoredBrowserGates.run(test, environment)
        console.log(`${result.passed ? 'PASS' : 'FAIL'} ${result.test} ${result.seconds}s`)
        results.push(result)
      }
    }
    const failed = results.filter(result => !result.passed).map(result => result.test)
    console.log(`ignored browser gates: ${results.length - failed.length} passed, ${failed.length} failed (${failed.join(' ') || 'none'})`)
    process.exitCode = failed.length > 0 ? 1 : 0
  }
}

await IgnoredBrowserGates.main()
