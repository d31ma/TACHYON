#!/usr/bin/env node
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseArgs } from 'node:util';
import { IsolatedServer } from './wasm-worker-measure/isolated-server.mjs';
import { ProbeFixture } from './wasm-worker-measure/probe-fixture.mjs';
import { ProbeRuntimes } from './wasm-worker-measure/probe-runtimes.mjs';
import { ProbeSession } from './wasm-worker-measure/probe-session.mjs';
import { ResultsTable } from './wasm-worker-measure/results-table.mjs';

class WasmWorkerMeasure {
  static usage = 'Usage: node scripts/wasm-worker-measure.mjs [--out <folder>] [--runs 3] [--throttles 1,4] [--runtimes js,rust,...] [--site <built website folder>] [--routes /,/docs] [--reuse-build]';
  static defaults = {
    out: path.join(os.tmpdir(), 'tachyon-wasm-worker-measure'),
    runs: '3',
    throttles: '1,4',
    runtimes: ProbeRuntimes.all.map((runtime) => runtime.key).join(','),
    routes: '/,/docs,/docs/features,/docs/first-app,/docs/features/companion-publish',
  };
  static listSeparator = ',';
  static fixtureFolder = 'fixture';
  static resultsJson = 'results.json';
  static resultsMarkdown = 'results.md';
  static releaseBinary = path.resolve(import.meta.dirname, '..', 'target', 'release', 'ty');

  /** @returns {Promise<void>} */
  static async run() {
    const options = WasmWorkerMeasure._options();
    const runtimes = ProbeRuntimes.chosen(options.runtimes);
    const project = WasmWorkerMeasure._prepare(options, runtimes);
    const fixture = await WasmWorkerMeasure._measureServed(ProbeFixture.output(project), (origin) => WasmWorkerMeasure._fixtureRecords(origin, runtimes, options));
    const site = await WasmWorkerMeasure._measureSite(options);
    const environment = WasmWorkerMeasure._environment(await ProbeSession.browserVersion(), options);
    WasmWorkerMeasure._report(options.out, { environment, runtimes: runtimes.map(({ key, label }) => ({ key, label })), fixture, site });
  }

  /** @returns {{out: string, runs: number, throttles: number[], runtimes: string[], site: string | undefined, routes: string[], reuseBuild: boolean}} */
  static _options() {
    const { values } = parseArgs({
      options: {
        out: { type: 'string', default: WasmWorkerMeasure.defaults.out },
        runs: { type: 'string', default: WasmWorkerMeasure.defaults.runs },
        throttles: { type: 'string', default: WasmWorkerMeasure.defaults.throttles },
        runtimes: { type: 'string', default: WasmWorkerMeasure.defaults.runtimes },
        site: { type: 'string' },
        routes: { type: 'string', default: WasmWorkerMeasure.defaults.routes },
        'reuse-build': { type: 'boolean', default: false },
        help: { type: 'boolean', default: false },
      },
    });
    WasmWorkerMeasure._stopForHelp(values.help);
    return {
      out: path.resolve(values.out),
      runs: Number(values.runs),
      throttles: values.throttles.split(WasmWorkerMeasure.listSeparator).map(Number),
      runtimes: values.runtimes.split(WasmWorkerMeasure.listSeparator),
      site: values.site && path.resolve(values.site),
      routes: values.routes.split(WasmWorkerMeasure.listSeparator),
      reuseBuild: values['reuse-build'],
    };
  }

  /**
   * @param {boolean} help
   * @returns {void}
   */
  static _stopForHelp(help) {
    if (!help) return;
    console.log(WasmWorkerMeasure.usage);
    process.exit(0);
  }

  /**
   * @param {{out: string, reuseBuild: boolean}} options
   * @param {typeof ProbeRuntimes.all} runtimes
   * @returns {string}
   */
  static _prepare(options, runtimes) {
    const project = path.join(options.out, WasmWorkerMeasure.fixtureFolder);
    if (options.reuseBuild && existsSync(ProbeFixture.output(project))) return project;
    rmSync(project, { recursive: true, force: true });
    ProbeFixture.write(project, runtimes);
    ProbeFixture.build(process.env.TAC_BIN ?? WasmWorkerMeasure.releaseBinary, project);
    return project;
  }

  /**
   * @template T
   * @param {string} root
   * @param {(origin: string) => Promise<T>} measure
   * @returns {Promise<T>}
   */
  static async _measureServed(root, measure) {
    const server = await IsolatedServer.start(root);
    try {
      return await measure(server.origin);
    } finally {
      await server.close();
    }
  }

  /**
   * @param {string} origin
   * @param {typeof ProbeRuntimes.all} runtimes
   * @param {{runs: number, throttles: number[]}} options
   * @returns {Promise<Array<Record<string, unknown>>>}
   */
  static async _fixtureRecords(origin, runtimes, options) {
    const records = [];
    for (const step of WasmWorkerMeasure._steps(runtimes, options)) {
      records.push(await WasmWorkerMeasure._runtimeStep(origin, step));
    }
    return records;
  }

  /**
   * @template T
   * @param {T[]} subjects
   * @param {{runs: number, throttles: number[]}} options
   * @returns {Array<{subject: T, throttle: number, run: number}>}
   */
  static _steps(subjects, options) {
    const runs = Array.from({ length: options.runs }, (_, index) => index + 1);
    return runs.flatMap((run) => options.throttles.flatMap((throttle) => subjects.map((subject) => ({ subject, throttle, run }))));
  }

  /**
   * @param {string} origin
   * @param {{subject: {key: string, service: string, className: string}, throttle: number, run: number}} step
   * @returns {Promise<Record<string, unknown>>}
   */
  static async _runtimeStep(origin, step) {
    const label = { runtime: step.subject.key, throttle: step.throttle, run: step.run };
    const record = { ...label, ...(await WasmWorkerMeasure._safely(() => ProbeSession.measureRuntime(origin, step.subject, step.throttle))) };
    console.log(JSON.stringify(record));
    return record;
  }

  /**
   * @param {{site: string | undefined, routes: string[], runs: number, throttles: number[]}} options
   * @returns {Promise<Array<Record<string, unknown>>>}
   */
  static async _measureSite(options) {
    if (!options.site) return [];
    return WasmWorkerMeasure._measureServed(options.site, (origin) => WasmWorkerMeasure._routeRecords(origin, options));
  }

  /**
   * @param {string} origin
   * @param {{routes: string[], runs: number, throttles: number[]}} options
   * @returns {Promise<Array<Record<string, unknown>>>}
   */
  static async _routeRecords(origin, options) {
    const records = [];
    for (const step of WasmWorkerMeasure._steps(options.routes, options)) {
      records.push(await WasmWorkerMeasure._routeStep(origin, step));
    }
    return records;
  }

  /**
   * @param {string} origin
   * @param {{subject: string, throttle: number, run: number}} step
   * @returns {Promise<Record<string, unknown>>}
   */
  static async _routeStep(origin, step) {
    const label = { route: step.subject, throttle: step.throttle, run: step.run };
    const record = { ...label, ...(await WasmWorkerMeasure._safely(() => ProbeSession.measureRoute(origin, step.subject, step.throttle))) };
    console.log(JSON.stringify(record));
    return record;
  }

  /**
   * @param {() => Promise<Record<string, unknown>>} measure
   * @returns {Promise<Record<string, unknown>>}
   */
  static async _safely(measure) {
    try {
      return await measure();
    } catch (error) {
      return { error: `${/** @type {Error} */ (error).message}`.split('\n')[0] };
    }
  }

  /**
   * @param {string} version
   * @param {{runs: number, throttles: number[]}} options
   * @returns {Record<string, unknown>}
   */
  static _environment(version, options) {
    return {
      date: new Date().toISOString(),
      browser: `Chromium ${version} (new headless mode, a fresh browser per measurement, service workers blocked, cross-origin isolated)`,
      machine: `${os.cpus()[0]?.model ?? os.arch()}, ${os.cpus().length} cores, ${Math.round(os.totalmem() / 2 ** 30)} GB`,
      'load average at the end': os.loadavg().map((value) => value.toFixed(1)).join(' '),
      runs: `${options.runs} per row (median shown)`,
      'CPU throttling': options.throttles.map((rate) => `${rate}x`).join(', '),
      ty: process.env.TAC_BIN ?? WasmWorkerMeasure.releaseBinary,
    };
  }

  /**
   * @param {string} out
   * @param {{environment: Record<string, unknown>, runtimes: Array<{key: string, label: string}>, fixture: Array<Record<string, unknown>>, site: Array<Record<string, unknown>>}} results
   * @returns {void}
   */
  static _report(out, results) {
    const markdown = ResultsTable.markdown(results);
    mkdirSync(out, { recursive: true });
    writeFileSync(path.join(out, WasmWorkerMeasure.resultsJson), JSON.stringify(results, null, 2));
    writeFileSync(path.join(out, WasmWorkerMeasure.resultsMarkdown), markdown);
    console.log(markdown);
  }
}

await WasmWorkerMeasure.run();
