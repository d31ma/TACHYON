class ProbePage {
  static longTaskThreshold = 50;
  static workMilliseconds = 200;
  static workerAnswerMilliseconds = 30_000;
  static workerFailure = 'the worker failed';
  static calibrationFloorMilliseconds = 40;
  static firstRounds = 1000;
  static roundsGrowth = 4;
  static mostRounds = 2_000_000_000;
  static microsecondsPerMillisecond = 1000;
  static workerUrl = '/__probe/probe-worker.js';
  static targetSize = { left: 8, top: 8, width: 160, height: 48 };
  /** @type {PerformanceEntry[]} */
  static longTasks = [];
  /** @type {PerformanceEntry[]} */
  static frames = [];
  /** @type {Array<{kind: string, at: number, delay: number}>} */
  static inputDelays = [];
  /** @type {Worker | null} */
  static worker = null;
  /** @type {Array<(answer: {error?: string, value?: unknown, fields?: Record<string, unknown>}) => void>} */
  static answers = [];
  /** @type {Promise<unknown>} */
  static working = Promise.resolve();
  /** @type {unknown} */
  static lastRead = null;

  /** @returns {void} */
  static observe() {
    ProbePage._watch({ type: 'longtask', buffered: true }, ProbePage.longTasks);
    ProbePage._watch({ type: 'long-animation-frame', buffered: true }, ProbePage.frames);
  }

  /**
   * @param {number} since
   * @returns {{count: number, totalMilliseconds: number, blockingMilliseconds: number, longestMilliseconds: number}}
   */
  static longTasksSince(since) {
    const durations = ProbePage._durationsSince(ProbePage.longTasks, since);
    return {
      count: durations.length,
      totalMilliseconds: ProbePage._sum(durations),
      blockingMilliseconds: ProbePage._sum(durations.map(ProbePage._blocking)),
      longestMilliseconds: Math.max(0, ...durations),
    };
  }

  /**
   * @param {number} since
   * @returns {Array<{url: string, milliseconds: number}>}
   */
  static scriptTimeSince(since) {
    return ProbePage._entriesSince(ProbePage.frames, since).flatMap(ProbePage._frameScripts);
  }

  /** @returns {string[]} */
  static loadedFiles() {
    return performance.getEntriesByType('resource').map((entry) => entry.name);
  }

  /** @returns {Promise<number>} */
  static async memoryBytes() {
    const measuring = /** @type {Performance & {measureUserAgentSpecificMemory: () => Promise<{bytes: number}>}} */ (performance);
    const measured = await measuring.measureUserAgentSpecificMemory();
    return measured.bytes;
  }

  /**
   * @param {string} service
   * @param {number} count
   * @returns {number[]}
   */
  static pageCalls(service, count) {
    const target = ProbePage._service(service);
    return ProbePage._repeat(count, () => ProbePage._timedPageCall(target));
  }

  /**
   * @param {string} service
   * @returns {number}
   */
  static calibrate(service) {
    const target = ProbePage._service(service);
    return ProbePage._roundsFor(target, ProbePage._growUntilMeasurable(target));
  }

  /**
   * @param {string} moduleUrl
   * @returns {Promise<number>}
   */
  static async startWorker(moduleUrl) {
    const started = performance.now();
    const worker = new Worker(ProbePage.workerUrl, { type: 'module' });
    worker.onmessage = (event) => ProbePage.answers.shift()?.(event.data);
    worker.onerror = ProbePage._workerFailed;
    ProbePage.worker = worker;
    ProbePage._requireAnswer(await ProbePage._ask({ op: 'start', url: moduleUrl }));
    return performance.now() - started;
  }

  /**
   * @param {number} count
   * @returns {Promise<number[]>}
   */
  static workerCalls(count) {
    return ProbePage._repeatInOrder(count, ProbePage._timedWorkerCall);
  }

  /** @returns {{x: number, y: number}} */
  static placeTargets() {
    const button = ProbePage._target('button', 'probe-target');
    const input = ProbePage._target('input', 'probe-input');
    input.style.top = `${ProbePage.targetSize.top * 2 + ProbePage.targetSize.height}px`;
    return { x: ProbePage.targetSize.left + ProbePage.targetSize.width / 2, y: ProbePage.targetSize.top + ProbePage.targetSize.height / 2 };
  }

  /**
   * @param {string} service
   * @param {number} rounds
   * @returns {number}
   */
  static startPageWork(service, rounds) {
    const target = ProbePage._service(service);
    ProbePage.working = new Promise((resolve) => setTimeout(() => resolve(target.spin(rounds)), 0));
    return performance.now();
  }

  /**
   * @param {number} rounds
   * @returns {number}
   */
  static startWorkerWork(rounds) {
    ProbePage.working = ProbePage._ask({ op: 'call', name: 'spin', args: [rounds] });
    return performance.now();
  }

  /** @returns {void} */
  static focusInput() {
    /** @type {HTMLInputElement} */ (document.getElementById('probe-input')).focus();
  }

  /**
   * @param {string} kind
   * @param {number} since
   * @returns {Promise<number | null>}
   */
  static async settle(kind, since) {
    await ProbePage.working;
    await ProbePage._nextFrame();
    return ProbePage._delayOf(kind, since);
  }

  /**
   * @param {string} name
   * @returns {{increment: () => number, count: number, spin: (rounds: number) => number}}
   */
  static _service(name) {
    const page = /** @type {typeof globalThis & {__tachyonTac: {service: (name: string) => {increment: () => number, count: number, spin: (rounds: number) => number}}}} */ (globalThis);
    return page.__tachyonTac.service(name);
  }

  /**
   * @param {PerformanceObserverInit & {type: string}} options
   * @param {PerformanceEntry[]} list
   * @returns {void}
   */
  static _watch(options, list) {
    if (!PerformanceObserver.supportedEntryTypes.includes(options.type)) return;
    new PerformanceObserver((entries) => list.push(...entries.getEntries())).observe(options);
  }

  /**
   * @param {PerformanceEntry[]} entries
   * @param {number} since
   * @returns {number[]}
   */
  static _durationsSince(entries, since) {
    return ProbePage._entriesSince(entries, since).map((entry) => entry.duration);
  }

  /**
   * @template {PerformanceEntry} T
   * @param {T[]} entries
   * @param {number} since
   * @returns {T[]}
   */
  static _entriesSince(entries, since) {
    return entries.filter((entry) => entry.startTime >= since);
  }

  /**
   * @param {ErrorEvent} event
   * @returns {void}
   */
  static _workerFailed(event) {
    ProbePage.answers.shift()?.({ error: event.message || ProbePage.workerFailure });
  }

  /**
   * @param {number} duration
   * @returns {number}
   */
  static _blocking(duration) {
    return Math.max(0, duration - ProbePage.longTaskThreshold);
  }

  /**
   * @param {number[]} values
   * @returns {number}
   */
  static _sum(values) {
    return values.reduce((total, value) => total + value, 0);
  }

  /**
   * @param {PerformanceEntry & {scripts?: Array<{sourceURL: string, duration: number}>}} frame
   * @returns {Array<{url: string, milliseconds: number}>}
   */
  static _frameScripts(frame) {
    return (frame.scripts ?? []).map((script) => ({ url: script.sourceURL, milliseconds: script.duration }));
  }

  /**
   * @param {number} count
   * @param {() => number} measure
   * @returns {number[]}
   */
  static _repeat(count, measure) {
    const durations = [];
    for (let index = 0; index < count; index += 1) {
      durations.push(measure());
    }
    return durations;
  }

  /**
   * @param {number} count
   * @param {() => Promise<number>} measure
   * @returns {Promise<number[]>}
   */
  static async _repeatInOrder(count, measure) {
    const durations = [];
    for (let index = 0; index < count; index += 1) {
      durations.push(await measure());
    }
    return durations;
  }

  /**
   * @param {{increment: () => number, count: number}} target
   * @returns {number}
   */
  static _timedPageCall(target) {
    const started = performance.now();
    target.increment();
    ProbePage._read(target.count);
    return (performance.now() - started) * ProbePage.microsecondsPerMillisecond;
  }

  /** @returns {Promise<number>} */
  static async _timedWorkerCall() {
    const started = performance.now();
    const answer = ProbePage._requireAnswer(await ProbePage._ask({ op: 'call', name: 'increment', args: [] }));
    ProbePage._read(answer.fields.count);
    return (performance.now() - started) * ProbePage.microsecondsPerMillisecond;
  }

  /**
   * @param {unknown} value
   * @returns {void}
   */
  static _read(value) {
    ProbePage.lastRead = value;
  }

  /**
   * @param {{spin: (rounds: number) => number}} target
   * @returns {{rounds: number, milliseconds: number}}
   */
  static _growUntilMeasurable(target) {
    let trial = { rounds: ProbePage.firstRounds, milliseconds: 0 };
    while (trial.milliseconds < ProbePage.calibrationFloorMilliseconds && trial.rounds < ProbePage.mostRounds) {
      trial = ProbePage._timedSpin(target, trial.rounds * ProbePage.roundsGrowth);
    }
    return trial;
  }

  /**
   * @param {{spin: (rounds: number) => number}} target
   * @param {number} rounds
   * @returns {{rounds: number, milliseconds: number}}
   */
  static _timedSpin(target, rounds) {
    const bounded = Math.min(rounds, ProbePage.mostRounds);
    const started = performance.now();
    target.spin(bounded);
    return { rounds: bounded, milliseconds: performance.now() - started };
  }

  /**
   * @param {{spin: (rounds: number) => number}} target
   * @param {{rounds: number, milliseconds: number}} trial
   * @returns {number}
   */
  static _roundsFor(target, trial) {
    return Math.min(ProbePage.mostRounds, Math.round(trial.rounds * ProbePage.workMilliseconds / trial.milliseconds));
  }

  /**
   * @param {object} message
   * @returns {Promise<{error?: string, value?: unknown, fields?: Record<string, unknown>}>}
   */
  static _ask(message) {
    return new Promise((resolve) => {
      const timer = setTimeout(() => resolve({ error: `the worker did not answer within ${ProbePage.workerAnswerMilliseconds} ms` }), ProbePage.workerAnswerMilliseconds);
      ProbePage.answers.push((answer) => {
        clearTimeout(timer);
        resolve(answer);
      });
      /** @type {Worker} */ (ProbePage.worker).postMessage(message);
    });
  }

  /**
   * @param {{error?: string, value?: unknown, fields?: Record<string, unknown>}} answer
   * @returns {{value?: unknown, fields: Record<string, unknown>}}
   */
  static _requireAnswer(answer) {
    if (answer.error) throw new Error(answer.error);
    return { value: answer.value, fields: answer.fields ?? {} };
  }

  /**
   * @param {string} tag
   * @param {string} id
   * @returns {HTMLElement}
   */
  static _target(tag, id) {
    const element = document.getElementById(id) ?? document.body.appendChild(document.createElement(tag));
    element.id = id;
    element.textContent = id;
    element.addEventListener('pointerdown', ProbePage._recordDelay);
    element.addEventListener('keydown', ProbePage._recordDelay);
    Object.assign(element.style, ProbePage._placement());
    return element;
  }

  /** @returns {Record<string, string>} */
  static _placement() {
    const size = ProbePage.targetSize;
    return { position: 'fixed', left: `${size.left}px`, top: `${size.top}px`, width: `${size.width}px`, height: `${size.height}px`, zIndex: '2147483647' };
  }

  /**
   * @param {Event} event
   * @returns {void}
   */
  static _recordDelay(event) {
    ProbePage.inputDelays.push({ kind: event.type, at: event.timeStamp, delay: performance.now() - event.timeStamp });
  }

  /** @returns {Promise<void>} */
  static _nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
  }

  /**
   * @param {string} kind
   * @param {number} since
   * @returns {number | null}
   */
  static _delayOf(kind, since) {
    const found = ProbePage.inputDelays.find((record) => record.kind === kind && record.at >= since);
    return found?.delay ?? null;
  }
}

Object.assign(globalThis, { ProbePage });
ProbePage.observe();
