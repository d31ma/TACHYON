import { RunStatistics } from './run-statistics.mjs';

export class ResultsTable {
  static missing = 'n/a';
  /** @type {Array<[string, string, number]>} */
  static startUpColumns = [
    ['ready ms', 'readyMilliseconds', 0],
    ['long tasks', 'longTasks', 0],
    ['blocking ms', 'blockingMilliseconds', 0],
    ['longest task ms', 'longestMilliseconds', 0],
    ['memory MB', 'memoryMegabytes', 1],
  ];
  /** @type {Array<[string, string, number]>} */
  static callColumns = [
    ['page mean us', 'callMeanMicroseconds', 1],
    ['page p50 us', 'callP50Microseconds', 0],
    ['page p95 us', 'callP95Microseconds', 0],
    ['worker mean us', 'workerCallMeanMicroseconds', 1],
    ['worker p50 us', 'workerCallP50Microseconds', 0],
    ['worker p95 us', 'workerCallP95Microseconds', 0],
    ['worker start ms', 'workerStartMilliseconds', 0],
    ['page blocking during worker start ms', 'workerStartBlockingMilliseconds', 0],
    ['memory with worker MB', 'workerMemoryMegabytes', 1],
  ];
  /** @type {Array<[string, string, number]>} */
  static inputColumns = [
    ['click delay page ms', 'pageClickDelayMilliseconds', 0],
    ['click delay worker ms', 'workerClickDelayMilliseconds', 0],
    ['key delay page ms', 'pageKeyDelayMilliseconds', 0],
    ['key delay worker ms', 'workerKeyDelayMilliseconds', 0],
    ['work rounds', 'rounds', 0],
  ];
  /** @type {Array<[string, string, number]>} */
  static routeColumns = [
    ['long tasks', 'longTasks', 0],
    ['blocking ms', 'blockingMilliseconds', 0],
    ['longest task ms', 'longestMilliseconds', 0],
    ['memory MB', 'memoryMegabytes', 1],
  ];

  /**
   * @param {{environment: Record<string, unknown>, runtimes: Array<{key: string, label: string}>, fixture: Array<Record<string, unknown>>, site: Array<Record<string, unknown>>}} results
   * @returns {string}
   */
  static markdown(results) {
    const groups = ResultsTable._groups(results.fixture, (record) => `${record.runtime}|${record.throttle}`);
    const routes = ResultsTable._groups(results.site, (record) => `${record.route}|${record.throttle}`);
    return [
      ResultsTable._environment(results.environment),
      ResultsTable._section('Start-up on the page thread (fixture routes)', ResultsTable._runtimeTable(groups, results.runtimes, ResultsTable.startUpColumns)),
      ResultsTable._section('One call: page thread against the worker prototype', ResultsTable._runtimeTable(groups, results.runtimes, ResultsTable.callColumns)),
      ResultsTable._section('Input while a service computes for 200 ms', ResultsTable._runtimeTable(groups, results.runtimes, ResultsTable.inputColumns)),
      ResultsTable._section('Website routes (page thread, script time from long animation frames)', ResultsTable._routeTable(routes)),
      ResultsTable._section('Errors', ResultsTable._errors([...results.fixture, ...results.site])),
    ].join('\n');
  }

  /**
   * @param {Record<string, unknown>} environment
   * @returns {string}
   */
  static _environment(environment) {
    return `# Tac WebAssembly services: page thread against Web Workers\n\n${Object.entries(environment).map(([name, value]) => `- ${name}: ${value}`).join('\n')}\n`;
  }

  /**
   * @param {string} title
   * @param {string} body
   * @returns {string}
   */
  static _section(title, body) {
    return `## ${title}\n\n${body}\n`;
  }

  /**
   * @param {Array<Record<string, unknown>>} records
   * @param {(record: Record<string, unknown>) => string} keyOf
   * @returns {Map<string, Array<Record<string, unknown>>>}
   */
  static _groups(records, keyOf) {
    const groups = new Map();
    for (const record of records) {
      groups.set(keyOf(record), [...(groups.get(keyOf(record)) ?? []), record]);
    }
    return groups;
  }

  /**
   * @param {Map<string, Array<Record<string, unknown>>>} groups
   * @param {Array<{key: string, label: string}>} runtimes
   * @param {Array<[string, string, number]>} columns
   * @returns {string}
   */
  static _runtimeTable(groups, runtimes, columns) {
    const header = ['runtime', 'CPU', ...columns.map(([title]) => title)];
    const rows = [...groups.entries()].map(([key, records]) => ResultsTable._runtimeRow(key, records, runtimes, columns));
    return ResultsTable._table(header, rows);
  }

  /**
   * @param {string} key
   * @param {Array<Record<string, unknown>>} records
   * @param {Array<{key: string, label: string}>} runtimes
   * @param {Array<[string, string, number]>} columns
   * @returns {string[]}
   */
  static _runtimeRow(key, records, runtimes, columns) {
    const [runtime, throttle] = key.split('|');
    const label = runtimes.find((candidate) => candidate.key === runtime)?.label ?? runtime;
    return [label, `${throttle}x`, ...columns.map((column) => ResultsTable._cell(records, column))];
  }

  /**
   * @param {Map<string, Array<Record<string, unknown>>>} routes
   * @returns {string}
   */
  static _routeTable(routes) {
    const header = ['route', 'CPU', 'runtimes loaded', ...ResultsTable.routeColumns.map(([title]) => title), 'script ms by runtime'];
    const rows = [...routes.entries()].map(([key, records]) => ResultsTable._routeRow(key, records));
    return ResultsTable._table(header, rows);
  }

  /**
   * @param {string} key
   * @param {Array<Record<string, unknown>>} records
   * @returns {string[]}
   */
  static _routeRow(key, records) {
    const [route, throttle] = key.split('|');
    const engines = /** @type {string[]} */ (records[0].engines ?? []).join(', ');
    return [route, `${throttle}x`, engines, ...ResultsTable.routeColumns.map((column) => ResultsTable._cell(records, column)), ResultsTable._scriptTime(records)];
  }

  /**
   * @param {Array<Record<string, unknown>>} records
   * @returns {string}
   */
  static _scriptTime(records) {
    const engines = new Set(records.flatMap((record) => Object.keys(ResultsTable._scriptTimes(record))));
    return [...engines].sort().map((engine) => `${engine} ${ResultsTable._scriptMedian(records, engine)}`).join('; ');
  }

  /**
   * @param {Array<Record<string, unknown>>} records
   * @param {string} engine
   * @returns {string}
   */
  static _scriptMedian(records, engine) {
    return ResultsTable._format(RunStatistics.median(records.map((record) => ResultsTable._scriptTimes(record)[engine] ?? 0)), 0);
  }

  /**
   * @param {Record<string, unknown>} record
   * @returns {Record<string, number>}
   */
  static _scriptTimes(record) {
    return /** @type {Record<string, number>} */ (record.scriptMilliseconds ?? {});
  }

  /**
   * @param {Array<Record<string, unknown>>} records
   * @param {[string, string, number]} column
   * @returns {string}
   */
  static _cell(records, [, field, digits]) {
    return ResultsTable._format(RunStatistics.median(records.map((record) => /** @type {number | null} */ (record[field] ?? null))), digits);
  }

  /**
   * @param {number | null} value
   * @param {number} digits
   * @returns {string}
   */
  static _format(value, digits) {
    if (value === null) return ResultsTable.missing;
    return value.toFixed(digits);
  }

  /**
   * @param {string[]} header
   * @param {string[][]} rows
   * @returns {string}
   */
  static _table(header, rows) {
    const divider = header.map(() => '---');
    return [header, divider, ...rows].map((cells) => `| ${cells.join(' | ')} |`).join('\n');
  }

  /**
   * @param {Array<Record<string, unknown>>} records
   * @returns {string}
   */
  static _errors(records) {
    const errors = new Set(records.flatMap(ResultsTable._errorLines));
    return [...errors].map((line) => `- ${line}`).join('\n') || '- none';
  }

  /**
   * @param {Record<string, unknown>} record
   * @returns {string[]}
   */
  static _errorLines(record) {
    const subject = record.runtime ?? record.route;
    return ['error', 'workerError'].filter((field) => record[field]).map((field) => `${subject} ${record.throttle}x ${field}: ${record[field]}`);
  }
}
