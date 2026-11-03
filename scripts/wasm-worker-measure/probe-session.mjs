import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { ProbeRuntimes } from './probe-runtimes.mjs';
import { RunStatistics } from './run-statistics.mjs';

export class ProbeSession {
  static calls = 300;
  static settleMilliseconds = 1000;
  static inputLeadMilliseconds = 30;
  static readyTimeoutMilliseconds = 120_000;
  static measurementTimeoutMilliseconds = 240_000;
  static readyPollingMilliseconds = 10;
  static bytesPerMegabyte = 1024 * 1024;
  static middle = 0.5;
  static tail = 0.95;
  static typedKey = 'a';
  static pageScript = path.join(path.dirname(fileURLToPath(import.meta.url)), 'probe-page.js');
  static browserChannel = 'chromium';
  static browserFlags = ['--enable-blink-features=ForceEagerMeasureMemory'];

  /** @returns {Promise<string>} */
  static async browserVersion() {
    const browser = await ProbeSession._launch();
    const version = browser.version();
    await browser.close();
    return version;
  }

  /**
   * @param {string} origin
   * @param {{key: string, service: string, className: string}} runtime
   * @param {number} throttle
   * @returns {Promise<Record<string, number | string | null>>}
   */
  static measureRuntime(origin, runtime, throttle) {
    return ProbeSession._inFreshBrowser(throttle, (page) => ProbeSession._runtimeRecord(page, origin, runtime));
  }

  /**
   * @param {string} origin
   * @param {string} route
   * @param {number} throttle
   * @returns {Promise<Record<string, unknown>>}
   */
  static measureRoute(origin, route, throttle) {
    return ProbeSession._inFreshBrowser(throttle, (page) => ProbeSession._routeRecord(page, `${origin}${route}`));
  }

  /**
   * @template T
   * @param {number} throttle
   * @param {(page: import('playwright').Page) => Promise<T>} measure
   * @returns {Promise<T>}
   */
  static async _inFreshBrowser(throttle, measure) {
    const browser = await ProbeSession._launch();
    const deadline = ProbeSession._deadline();
    try {
      const page = await ProbeSession._open(browser, throttle);
      return await Promise.race([ProbeSession._quietWhenLate(measure(page)), deadline.expired]);
    } finally {
      clearTimeout(deadline.timer);
      await browser.close();
    }
  }

  /** @returns {Promise<import('playwright').Browser>} */
  static _launch() {
    return chromium.launch({ channel: ProbeSession.browserChannel, args: ProbeSession.browserFlags });
  }

  /** @returns {{expired: Promise<never>, timer: NodeJS.Timeout | undefined}} */
  static _deadline() {
    /** @type {NodeJS.Timeout | undefined} */
    let timer;
    /** @type {Promise<never>} */
    const expired = new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`the measurement took longer than ${ProbeSession.measurementTimeoutMilliseconds} ms`)), ProbeSession.measurementTimeoutMilliseconds);
    });
    return { expired, timer };
  }

  /**
   * @template T
   * @param {Promise<T>} measuring
   * @returns {Promise<T>}
   */
  static _quietWhenLate(measuring) {
    measuring.catch(() => undefined);
    return measuring;
  }

  /**
   * @param {import('playwright').Browser} browser
   * @param {number} throttle
   * @returns {Promise<import('playwright').Page>}
   */
  static async _open(browser, throttle) {
    const context = await browser.newContext({ serviceWorkers: 'block' });
    await context.addInitScript({ path: ProbeSession.pageScript });
    const page = await context.newPage();
    const devtools = await context.newCDPSession(page);
    await devtools.send('Emulation.setCPUThrottlingRate', { rate: throttle });
    return page;
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} origin
   * @param {{key: string, service: string, className: string}} runtime
   * @returns {Promise<Record<string, number | string | null>>}
   */
  static async _runtimeRecord(page, origin, runtime) {
    const load = await ProbeSession._load(page, `${origin}/${runtime.key}/`, runtime.service);
    const calls = ProbeSession._percentiles('call', await ProbeSession._probe(page, 'pageCalls', runtime.service, ProbeSession.calls));
    const rounds = await ProbeSession._probe(page, 'calibrate', runtime.service);
    const point = await ProbeSession._probe(page, 'placeTargets');
    const input = await ProbeSession._input(page, point, () => ProbeSession._probe(page, 'startPageWork', runtime.service, rounds));
    const worker = await ProbeSession._worker(page, `${origin}/.tachyon/services/${runtime.className}.js`, point, rounds);
    return { ...load, ...calls, rounds, ...ProbeSession._named('page', input), ...worker };
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} url
   * @param {string} service
   * @returns {Promise<Record<string, number | null>>}
   */
  static async _load(page, url, service) {
    await page.goto(url);
    await page.waitForFunction(ProbeSession._serviceReady, service, {
      polling: ProbeSession.readyPollingMilliseconds,
      timeout: ProbeSession.readyTimeoutMilliseconds,
    });
    const readyMilliseconds = await page.evaluate(() => performance.now());
    await page.waitForTimeout(ProbeSession.settleMilliseconds);
    return { readyMilliseconds, ...(await ProbeSession._pageState(page)) };
  }

  /**
   * @param {import('playwright').Page} page
   * @returns {Promise<Record<string, number>>}
   */
  static async _pageState(page) {
    const tasks = await ProbeSession._probe(page, 'longTasksSince', 0);
    return {
      longTasks: tasks.count,
      blockingMilliseconds: tasks.blockingMilliseconds,
      longestMilliseconds: tasks.longestMilliseconds,
      memoryMegabytes: await ProbeSession._memoryMegabytes(page),
    };
  }

  /**
   * @param {import('playwright').Page} page
   * @returns {Promise<number>}
   */
  static async _memoryMegabytes(page) {
    return (await ProbeSession._probe(page, 'memoryBytes')) / ProbeSession.bytesPerMegabyte;
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} moduleUrl
   * @param {{x: number, y: number}} point
   * @param {number} rounds
   * @returns {Promise<Record<string, number | string | null>>}
   */
  static async _worker(page, moduleUrl, point, rounds) {
    try {
      return await ProbeSession._workerRecord(page, moduleUrl, point, rounds);
    } catch (error) {
      return { workerError: `${/** @type {Error} */ (error).message}`.split('\n')[0] };
    }
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} moduleUrl
   * @param {{x: number, y: number}} point
   * @param {number} rounds
   * @returns {Promise<Record<string, number | null>>}
   */
  static async _workerRecord(page, moduleUrl, point, rounds) {
    const since = await page.evaluate(() => performance.now());
    const workerStartMilliseconds = await ProbeSession._probe(page, 'startWorker', moduleUrl);
    const start = await ProbeSession._probe(page, 'longTasksSince', since);
    const calls = ProbeSession._percentiles('workerCall', await ProbeSession._probe(page, 'workerCalls', ProbeSession.calls));
    const input = await ProbeSession._input(page, point, () => ProbeSession._probe(page, 'startWorkerWork', rounds));
    return {
      workerStartMilliseconds,
      workerStartBlockingMilliseconds: start.blockingMilliseconds,
      ...calls,
      ...ProbeSession._named('worker', input),
      workerMemoryMegabytes: await ProbeSession._memoryMegabytes(page),
    };
  }

  /**
   * @param {import('playwright').Page} page
   * @param {{x: number, y: number}} point
   * @param {() => Promise<number>} startWork
   * @returns {Promise<Record<string, number | null>>}
   */
  static async _input(page, point, startWork) {
    const click = await ProbeSession._clickDuring(page, point, startWork);
    const key = await ProbeSession._keyDuring(page, startWork);
    return { ClickDelayMilliseconds: click, KeyDelayMilliseconds: key };
  }

  /**
   * @param {import('playwright').Page} page
   * @param {{x: number, y: number}} point
   * @param {() => Promise<number>} startWork
   * @returns {Promise<number | null>}
   */
  static async _clickDuring(page, point, startWork) {
    const since = await startWork();
    await page.waitForTimeout(ProbeSession.inputLeadMilliseconds);
    await page.mouse.click(point.x, point.y);
    return ProbeSession._probe(page, 'settle', 'pointerdown', since);
  }

  /**
   * @param {import('playwright').Page} page
   * @param {() => Promise<number>} startWork
   * @returns {Promise<number | null>}
   */
  static async _keyDuring(page, startWork) {
    await ProbeSession._probe(page, 'focusInput');
    const since = await startWork();
    await page.waitForTimeout(ProbeSession.inputLeadMilliseconds);
    await page.keyboard.press(ProbeSession.typedKey);
    return ProbeSession._probe(page, 'settle', 'keydown', since);
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} url
   * @returns {Promise<Record<string, unknown>>}
   */
  static async _routeRecord(page, url) {
    await page.goto(url, { waitUntil: 'networkidle' });
    await page.waitForTimeout(ProbeSession.settleMilliseconds);
    const state = await ProbeSession._pageState(page);
    const files = await ProbeSession._probe(page, 'loadedFiles');
    const scripts = await ProbeSession._probe(page, 'scriptTimeSince', 0);
    return { ...state, engines: ProbeSession._enginesIn(files), scriptMilliseconds: ProbeSession._timeByEngine(scripts) };
  }

  /**
   * @param {string[]} files
   * @returns {string[]}
   */
  static _enginesIn(files) {
    const engines = new Set(files.map(ProbeRuntimes.engineOf));
    engines.delete(ProbeRuntimes.otherEngine);
    return [...engines].sort();
  }

  /**
   * @param {Array<{url: string, milliseconds: number}>} scripts
   * @returns {Record<string, number>}
   */
  static _timeByEngine(scripts) {
    /** @type {Record<string, number>} */
    const totals = {};
    for (const script of scripts) {
      const engine = ProbeRuntimes.engineOf(script.url);
      totals[engine] = (totals[engine] ?? 0) + script.milliseconds;
    }
    return totals;
  }

  /**
   * @param {string} prefix
   * @param {number[]} durations
   * @returns {Record<string, number | null>}
   */
  static _percentiles(prefix, durations) {
    const measured = RunStatistics.withoutWarmUp(durations);
    return {
      [`${prefix}MeanMicroseconds`]: RunStatistics.mean(measured),
      [`${prefix}P50Microseconds`]: RunStatistics.percentile(measured, ProbeSession.middle),
      [`${prefix}P95Microseconds`]: RunStatistics.percentile(measured, ProbeSession.tail),
    };
  }

  /**
   * @param {string} prefix
   * @param {Record<string, number | null>} values
   * @returns {Record<string, number | null>}
   */
  static _named(prefix, values) {
    return Object.fromEntries(Object.entries(values).map(([name, value]) => [`${prefix}${name}`, value]));
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} method
   * @param {...unknown} values
   * @returns {Promise<any>}
   */
  static _probe(page, method, ...values) {
    return page.evaluate(ProbeSession._callProbe, { method, values });
  }

  /**
   * @param {{method: string, values: unknown[]}} request
   * @returns {unknown}
   */
  static _callProbe({ method, values }) {
    const page = /** @type {{ProbePage: Record<string, (...values: unknown[]) => unknown>}} */ (/** @type {unknown} */ (globalThis));
    return page.ProbePage[method](...values);
  }

  /**
   * @param {string} service
   * @returns {boolean}
   */
  static _serviceReady(service) {
    const page = /** @type {{__tachyonTac?: {service: (name: string) => unknown}}} */ (/** @type {unknown} */ (globalThis));
    return Boolean(page.__tachyonTac?.service(service));
  }
}
