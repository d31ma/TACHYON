class NativePreviewReport {
  /** @type {number} */
  static #WAIT_MILLISECONDS = 20000;
  /** @type {number} */
  static #POLL_MILLISECONDS = 80;
  /** @type {number} */
  static #HOLD_MILLISECONDS = 6500;
  /** @type {number} */
  static #REQUIRED_POLLS = 3;
  /** @type {number} */
  static #REQUIRED_GROUPS = 2;
  /** @type {number} */
  static #WIDE_WIDTH = 1100;
  /** @type {number} */
  static #FULL_WIDTH = 1600;
  /** @type {number} */
  static #MAXIMUM_COLUMNS = 4;
  /** @type {number} */
  static #SCROLL_DISTANCE = 120;
  /** @type {number} */
  static #MAXIMUM_ZOOM = 1.5;
  /** @type {number} */
  static #SIZE_TOLERANCE = 1;
  /** @type {number} */
  static #FIRST_SAMPLE = 1;
  /** @type {string} */
  static #SAMPLE_SELECTOR = '.main-footer > span:last-child';
  /** @type {string} */
  static #GROUP_SELECTOR = '.graph-group';
  /** @type {string} */
  static #SCROLL_SELECTOR = '.group-graph-scroll';
  /** @type {string} */
  static #TABLE_GRID = '.session-list';
  /** @type {string} */
  static #TABLE_CARD = '.table-branch';
  /** @type {string} */
  static #HISTORY_GRID = '.history-grid';
  /** @type {string} */
  static #HISTORY_CARD = '.history-card';

  /** @param {{label: string, action: string, section: string, mode: string, width: number, height: number, check: string}} input @param {boolean} iconLoaded @returns {Promise<string>} */
  static async verify(input, iconLoaded) {
    await this.#navigation();
    const before = this.#samples();
    await this.#perform(input.action);
    await this.#settle(input, before);
    await this.#finishCondition(input);
    await this.#captureInteraction(input.check);
    const report = this.#report(iconLoaded);
    this.#assertReport(report, input);
    this.#assertTabs(input.check);
    return JSON.stringify({...report, actionHistory: this.#actionRecords(), verification: {success: true, label: input.label, requested: input}});
  }

  /** @returns {Promise<void>} */
  static async #navigation() {
    const started = performance.now();
    await this.#waitNavigation(started);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitNavigation(started) {
    while (document.querySelectorAll('.nav-item').length === 0) await this.#waitIteration(started);
  }

  /** @param {boolean} iconLoaded @returns {string} */
  static read(iconLoaded) {
    return JSON.stringify(this.#report(iconLoaded));
  }

  /** @param {string} action @returns {Promise<void>} */
  static async #perform(action) {
    switch (action) {
      case 'running': this.section("What's running"); return;
      case 'table': this.table(); return;
      case 'tree': this.processes(); return;
      case 'connector-first': await this.connector('codex:61685:1790794014:557775'); return;
      case 'connector-second': await this.connector('codex:61639:1790794008:120943'); return;
      case 'exercise': await this.exercise(); return;
      case 'pause': this.pause(); return;
      case 'resume': this.resume(); return;
      case 'history': this.section('Resource history'); return;
      case 'activity': this.section('Activity'); return;
      case 'runtimes': this.section('Runtimes'); return;
      case 'polls': return;
      case 'hold': return;
      default: throw new Error('Unknown native QA action: ' + action);
    }
  }

  /** @param {string} name @returns {void} */
  static section(name) {
    this.#click('.nav-item', name);
  }

  /** @returns {void} */
  static table() {
    this.section('Processes');
    this.#click('.view-tabs w-btn', 'Table');
  }

  /** @returns {void} */
  static processes() {
    this.section('Processes');
    this.#click('.view-tabs w-btn', 'Tree');
  }

  /** @param {string} identifier @returns {Promise<void>} */
  static async connector(identifier) {
    this.processes();
    await this.#waitConnector(identifier, performance.now());
    await this.#selectGroup(identifier);
  }

  /** @param {string} identifier @param {number} started @returns {Promise<void>} */
  static async #waitConnector(identifier, started) {
    while (!this.#connectorReady(identifier)) await this.#waitIteration(started);
  }

  /** @param {string} identifier @returns {boolean} */
  static #connectorReady(identifier) {
    return Array.from(document.querySelectorAll('[role=tab][data-graph-group]')).some(
      /** @param {Element} tab @returns {boolean} */
      tab => /** @type {HTMLElement} */ (tab).dataset.graphGroup === identifier
    );
  }

  /** @returns {void} */
  static pause() {
    this.#click('.status-controls w-btn', 'Pause');
  }

  /** @returns {void} */
  static resume() {
    this.#click('.status-controls w-btn', 'Resume');
  }

  /** @returns {Promise<void>} */
  static async exercise() {
    const ids = this.#groupIds();
    this.#requireGroups(ids.length);
    const before = await this.#observeGroups(ids);
    await this.#selectGroup(ids[0]);
    this.#zoomFirst(this.#groups()[0]);
    await this.#frames();
    const after = await this.#observeGroups(ids);
    await this.#selectGroup(ids[0]);
    const scroll = this.#overflowScroll();
    this.#scroll(scroll);
    document.body.dataset.qaBaseline = JSON.stringify({before, after, scroll: this.#scrollRecord(scroll)});
  }

  /** @returns {string[]} */
  static #groupIds() {
    return Array.from(document.querySelectorAll('[role=tab][data-graph-group]')).slice(0, this.#REQUIRED_GROUPS).map(
      /** @param {Element} tab @returns {string} */
      tab => String(/** @type {HTMLElement} */ (tab).dataset.graphGroup)
    );
  }

  /** @param {string} id @returns {Promise<void>} */
  static async #selectGroup(id) {
    const tab = Array.from(document.querySelectorAll('[role=tab][data-graph-group]')).find(
      /** @param {Element} item @returns {boolean} */
      item => /** @type {HTMLElement} */ (item).dataset.graphGroup === id
    );
    this.#press(tab, 'process group ' + id);
    await this.#waitGroup(id, performance.now());
    await this.#frames();
  }

  /** @param {string} id @param {number} started @returns {Promise<void>} */
  static async #waitGroup(id, started) {
    while (this.#groups().map(group => this.#scale(group).id).join() !== id) await this.#waitIteration(started);
  }

  /** @param {string[]} ids @returns {Promise<{id: string, scale: string}[]>} */
  static async #observeGroups(ids) {
    /** @type {{id: string, scale: string}[]} */
    const observed = [];
    for (const id of ids) { await this.#selectGroup(id); observed.push(this.#scale(this.#groups()[0])); }
    return observed;
  }

  /** @param {string} check @returns {Promise<void>} */
  static async #captureInteraction(check) {
    if (!['zoom', 'polls'].includes(check)) return;
    const current = await this.#observeGroups(this.#groupIds());
    document.body.dataset.qaCurrent = JSON.stringify(current);
    await this.#selectGroup(current[0].id);
  }

  /** @returns {Element[]} */
  static #groups() {
    return Array.from(document.querySelectorAll(this.#GROUP_SELECTOR)).slice(0, this.#REQUIRED_GROUPS);
  }

  /** @param {number} count @returns {void} */
  static #requireGroups(count) {
    if (count < this.#REQUIRED_GROUPS) throw new Error('Native QA needs two actual process groups for independent zoom.');
  }

  /** @param {Element} group @returns {void} */
  static #zoomFirst(group) {
    const current = this.#scale(group);
    const level = Number.parseFloat(current.scale.replace('scale(', ''));
    const action = this.#zoomAction(level);
    const button = group.querySelector('[data-zoom="' + action + '"]');
    this.#press(button, 'group zoom ' + action);
  }

  /** @param {number} level @returns {string} */
  static #zoomAction(level) {
    if (level >= this.#MAXIMUM_ZOOM) return 'out';
    return 'in';
  }

  /** @param {Element} group @returns {{id: string, scale: string}} */
  static #scale(group) {
    const canvas = /** @type {HTMLElement | null} */ (group.querySelector('.graph-canvas'));
    return {id: String(/** @type {HTMLElement} */ (group).dataset.group), scale: this.#transform(canvas)};
  }

  /** @param {HTMLElement | null} canvas @returns {string} */
  static #transform(canvas) {
    if (canvas === null) return '';
    return canvas.style.transform;
  }

  /** @returns {HTMLElement | null} */
  static #overflowScroll() {
    const element = Array.from(document.querySelectorAll(this.#SCROLL_SELECTOR)).find(
      /** @param {Element} item @returns {boolean} */
      item => [item.scrollHeight > item.clientHeight, item.scrollWidth > item.clientWidth].some(Boolean)
    );
    return this.#element(element);
  }

  /** @param {Element | undefined} element @returns {HTMLElement | null} */
  static #element(element) {
    return /** @type {HTMLElement | null} */ (element ?? null);
  }

  /** @param {HTMLElement | null} scroll @returns {void} */
  static #scroll(scroll) {
    if (scroll === null) return;
    scroll.scrollTop = Math.min(this.#SCROLL_DISTANCE, scroll.scrollHeight - scroll.clientHeight);
    scroll.scrollLeft = Math.min(this.#SCROLL_DISTANCE, scroll.scrollWidth - scroll.clientWidth);
    scroll.dispatchEvent(new Event('scroll'));
  }

  /** @param {HTMLElement | null} scroll @returns {{id: string, top: number, left: number} | null} */
  static #scrollRecord(scroll) {
    if (scroll === null) return null;
    return {id: String(scroll.dataset.group), top: scroll.scrollTop, left: scroll.scrollLeft};
  }

  /** @param {string} selector @param {string} text @returns {void} */
  static #click(selector, text) {
    const button = Array.from(document.querySelectorAll(selector)).find(
      /** @param {Element} item @returns {boolean} */
      item => String(item.textContent).includes(text)
    );
    this.#press(button, text);
  }

  /** @param {Element | null | undefined} button @param {string} label @returns {void} */
  static #press(button, label) {
    if (!button) throw new Error('Native QA control is missing: ' + label);
    /** @type {HTMLElement} */ (button).click();
  }

  /** @param {{section: string, mode: string, width: number, height: number, check: string}} input @param {number} before @returns {Promise<void>} */
  static async #settle(input, before) {
    const started = performance.now();
    await this.#wait(input, before, started);
    await this.#frames();
    this.#requireReady(this.#ready(input, before), input.section);
  }

  /** @param {{section: string, mode: string, width: number, height: number, check: string}} input @param {number} before @param {number} started @returns {Promise<void>} */
  static async #wait(input, before, started) {
    while (!this.#ready(input, before)) await this.#waitIteration(started);
  }

  /** @param {number} started @returns {Promise<void>} */
  static async #waitIteration(started) {
    this.#requireTime(started);
    this.#requireNative();
    /** @type {Promise<void>} */
    const waiting = new Promise(
      /** @param {() => void} resolve @returns {void} */
      resolve => { setTimeout(resolve, this.#POLL_MILLISECONDS); }
    );
    await waiting;
  }

  /** @param {number} started @returns {void} */
  static #requireTime(started) {
    if (performance.now() - started >= this.#WAIT_MILLISECONDS) throw new Error('Native QA timed out waiting for the requested view, dimensions or samples.');
  }

  /** @returns {void} */
  static #requireNative() {
    const failure = document.querySelector('[data-native-failure]');
    if (failure) throw new Error('Native app reported a failure: ' + String(failure.getAttribute('data-native-failure')).slice(0, 300));
  }

  /** @returns {Promise<void>} */
  static #frames() {
    return new Promise(
      /** @param {() => void} resolve @returns {void} */
      resolve => { requestAnimationFrame(() => { requestAnimationFrame(resolve); }); }
    );
  }

  /** @param {{section: string, mode: string, width: number, height: number, check: string}} input @param {number} before @returns {boolean} */
  static #ready(input, before) {
    const section = this.#text(document.querySelector('.nav-item.selected'));
    const mode = this.#class(document.querySelector(this.#TABLE_GRID));
    return [section.includes(input.section), this.#modeReady(input.mode, mode),
      this.#viewportReady(input),
      this.#conditionReady(input.check, before)].every(Boolean);
  }

  /** @param {{width: number, height: number, check: string}} input @returns {boolean} */
  static #viewportReady(input) {
    if (input.check === 'updates') return true;
    return [Math.abs(innerWidth - input.width) <= this.#SIZE_TOLERANCE,
      Math.abs(innerHeight - input.height) <= this.#SIZE_TOLERANCE].every(Boolean);
  }

  /** @param {string} expected @param {string} actual @returns {boolean} */
  static #modeReady(expected, actual) {
    if (expected === '') return true;
    return actual.includes('mode-' + expected);
  }

  /** @param {Element | null} element @returns {string} */
  static #class(element) {
    if (element === null) return '';
    return String(element.className);
  }

  /** @param {string} check @param {number} before @returns {boolean} */
  static #conditionReady(check, before) {
    switch (check) {
      case 'polls': return this.#samples() >= before + this.#REQUIRED_POLLS;
      case 'updates': return this.#samples() >= before + this.#REQUIRED_POLLS;
      case 'paused': return this.#live().toLowerCase().includes('paused');
      case 'resumed': return [!this.#live().toLowerCase().includes('paused'), this.#samples() > Number(document.body.dataset.qaPausedSamples)].every(Boolean);
      case 'tree': return document.querySelectorAll('.graph-node').length > 0;
      case 'history': return document.querySelectorAll(this.#HISTORY_CARD).length > 0;
      case 'logical': return this.#logicalRecords().filter(record => record.status !== 'stale').length >= 3;
      case 'action-history': return this.#historyComplete();
      case 'action-running': return this.#runningAction() !== undefined;
      case 'action-progress': return [this.#samples() >= before + this.#REQUIRED_POLLS, this.#actionAdvanced()].every(Boolean);
      default: return this.#samples() >= this.#FIRST_SAMPLE;
    }
  }

  /** @returns {boolean} */
  static #historyComplete() {
    const expected = document.body.dataset.qaActionId;
    const rows = Array.from(document.querySelectorAll('.action-history-row')).filter(row => this.#expectedAction(row, expected));
    return rows.some(this.#completedHistory);
  }

  /** @param {Element} row @param {string | undefined} expected @returns {boolean} */
  static #expectedAction(row, expected) {
    if (expected === undefined) return true;
    return /** @type {HTMLElement} */ (row).dataset.action === expected;
  }

  /** @returns {HTMLElement | undefined} */
  static #runningAction() {
    return Array.from(document.querySelectorAll('.action-history-row'), row => /** @type {HTMLElement} */ (row)).find(
      row => [this.#text(row.querySelector('.action-history-heading strong')) === 'offline_ui_validation',
        this.#text(row.querySelector('.status-pill')) === 'running', row.querySelector('.action-elapsed') !== null].every(Boolean)
    );
  }

  /** @returns {boolean} */
  static #actionAdvanced() {
    const row = this.#runningAction();
    return [row !== undefined, row?.dataset.action === document.body.dataset.qaActionId,
      this.#text(row?.querySelector('.action-elapsed') ?? null) !== document.body.dataset.qaActionElapsed].every(Boolean);
  }

  /** @returns {{id: string | undefined, agent: string | undefined, name: string, status: string, elapsed: string, text: string}[]} */
  static #actionRecords() {
    return Array.from(document.querySelectorAll('.action-history-row'), row => ({
      id: /** @type {HTMLElement} */ (row).dataset.action,
      agent: /** @type {HTMLElement | null} */ (row.closest('.activity-card'))?.dataset.agent,
      name: this.#text(row.querySelector('.action-history-heading strong')),
      status: this.#text(row.querySelector('.status-pill')), elapsed: this.#text(row.querySelector('.action-elapsed')), text: this.#text(row)
    }));
  }

  /** @param {Element} row @returns {boolean} */
  static #completedHistory(row) {
    const text = String(row.textContent);
    return ['offline_ui_validation', 'completed', 'Measured by the reporting caller'].every(
      /** @param {string} value @returns {boolean} */
      value => text.includes(value)
    );
  }

  /** @param {boolean} ready @param {string} section @returns {void} */
  static #requireReady(ready, section) {
    if (!ready) throw new Error('Native QA view changed before capture: ' + section);
  }

  /** @param {{check: string}} input @returns {Promise<void>} */
  static async #finishCondition(input) {
    await this.#holdWhenNeeded(input.check);
    this.#rememberPause(input.check);
    this.#rememberAction(input.check);
    this.#requireNative();
  }

  /** @param {string} check @returns {void} */
  static #rememberAction(check) {
    if (check !== 'action-running') return;
    const row = this.#runningAction();
    document.body.dataset.qaActionId = row?.dataset.action;
    document.body.dataset.qaActionElapsed = this.#text(row?.querySelector('.action-elapsed') ?? null);
  }

  /** @param {string} check @returns {Promise<void>} */
  static async #holdWhenNeeded(check) {
    if (check !== 'hold') return;
    /** @type {Promise<void>} */
    const waiting = new Promise(
      /** @param {() => void} resolve @returns {void} */
      resolve => { setTimeout(resolve, this.#HOLD_MILLISECONDS); }
    );
    await waiting;
  }

  /** @param {string} check @returns {void} */
  static #rememberPause(check) {
    if (check !== 'paused') return;
    document.body.dataset.qaPausedSamples = String(this.#samples());
  }

  /** @param {boolean} iconLoaded @returns {{capturedAt: string, native: string | undefined, section: string, samples: string, sampleCount: number, width: number, height: number, horizontalOverflow: boolean, documentScroll: boolean, live: string, groups: number, graphGroups: number, graphNodes: number, graphTabBounds: {id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}[], tableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, historyLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, interaction: {baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null, chartPoints: number | null, chartUnit: string | null, sessionContainers: number, flatAgentCards: number, historyCards: number, runtimeCards: number, activityCards: number, sidebarOverflow: boolean, summaryBounds: {text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}[], logicalRecords: {id: string, name: string, status: string, source: string, task: string}[], logicalRows: number, appIconLoaded: boolean}} */
  static #report(iconLoaded) {
    const canvas = document.querySelector('.resource-chart');
    return {capturedAt: new Date().toISOString(), native: document.body.dataset.platform,
      section: this.#text(document.querySelector('.nav-item.selected')),
      samples: this.#text(document.querySelector(this.#SAMPLE_SELECTOR)), sampleCount: this.#samples(),
      width: innerWidth, height: innerHeight, horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
      documentScroll: document.documentElement.scrollHeight > innerHeight, live: this.#live(),
      groups: document.querySelectorAll('.session-row').length,
      graphGroups: document.querySelectorAll(this.#GROUP_SELECTOR).length, graphNodes: document.querySelectorAll('.graph-node').length, graphTabBounds: this.#tabBounds(),
      tableLayout: this.#grid(this.#TABLE_GRID, this.#TABLE_CARD), historyLayout: this.#grid(this.#HISTORY_GRID, this.#HISTORY_CARD),
      interaction: this.#interaction(), chartPoints: this.#points(canvas), chartUnit: this.#chartUnit(canvas),
      sessionContainers: document.querySelectorAll('.session-card').length, flatAgentCards: document.querySelectorAll('.agent-grid > .logical-agent-row').length,
      historyCards: document.querySelectorAll(this.#HISTORY_CARD).length, runtimeCards: document.querySelectorAll('.runtime-card').length,
      activityCards: document.querySelectorAll('.activity-card').length, sidebarOverflow: this.#sidebarOverflow(),
      summaryBounds: this.#summaryBounds(), logicalRecords: this.#logicalRecords(), logicalRows: document.querySelectorAll('.logical-agent-row').length, appIconLoaded: iconLoaded};
  }

  /** @returns {{id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}[]} */
  static #tabBounds() {
    return Array.from(document.querySelectorAll('[role=tab][data-graph-group]')).filter(tab => this.#visible(tab)).map(tab => this.#tabBound(tab));
  }

  /** @param {Element} tab @returns {{id: string, height: number, textTop: number, textBottom: number, top: number, bottom: number, lineHeight: string, appearance: string}} */
  static #tabBound(tab) {
    return {...this.#elementTextBound(tab), id: String(/** @type {HTMLElement} */ (tab).dataset.graphGroup),
      appearance: getComputedStyle(tab).appearance};
  }

  /** @param {string} check @returns {void} */
  static #assertTabs(check) {
    if (check !== 'tree') return;
    this.#requireTabs(this.#tabBounds());
  }

  /** @param {{height: number, textTop: number, textBottom: number, top: number, bottom: number}[]} tabs @returns {void} */
  static #requireTabs(tabs) {
    const ready = tabs.every(tab => [tab.height >= 36, tab.textTop >= tab.top, tab.textBottom <= tab.bottom].every(Boolean));
    if (!ready) throw new Error('Native QA process tab text is clipped or its button is too short.');
  }

  /** @returns {{text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}[]} */
  static #summaryBounds() {
    return Array.from(document.querySelectorAll('.summary-grid .metric-label')).filter(label => this.#visible(label)).map(label => this.#summaryBound(label));
  }

  /** @param {Element} label @returns {{text: string, top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string, overflow: string}} */
  static #summaryBound(label) {
    return {...this.#elementTextBound(label), text: this.#text(label), overflow: getComputedStyle(label).overflow};
  }

  /** @param {Element} label @returns {{top: number, bottom: number, textTop: number, textBottom: number, height: number, lineHeight: string}} */
  static #elementTextBound(label) {
    const rectangle = label.getBoundingClientRect();
    const range = document.createRange();
    range.selectNodeContents(label);
    const text = range.getBoundingClientRect();
    const style = getComputedStyle(label);
    return {top: rectangle.top, bottom: rectangle.bottom,
      textTop: text.top, textBottom: text.bottom, height: rectangle.height, lineHeight: style.lineHeight};
  }

  /** @returns {{id: string, name: string, status: string, source: string, task: string}[]} */
  static #logicalRecords() {
    return Array.from(document.querySelectorAll('.logical-agent-row')).map(
      /** @param {Element} row @returns {{id: string, name: string, status: string, source: string, task: string}} */
      row => ({id: this.#attribute(row, 'data-agent') ?? this.#attribute(row.querySelector('[data-agent]'), 'data-agent') ?? '',
        name: this.#text(row.querySelector('h3')), status: this.#text(row.querySelector('.status-pill')),
        source: this.#text(row.querySelector('.feed-time')), task: this.#text(row.querySelector('.assignment strong'))})
    );
  }

  /** @param {string} gridSelector @param {string} cardSelector @returns {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} */
  static #grid(gridSelector, cardSelector) {
    const grid = document.querySelector(gridSelector);
    const columns = this.#columns(grid);
    const cards = Array.from(document.querySelectorAll(gridSelector + ' ' + cardSelector)).filter(
      /** @param {Element} card @returns {boolean} */
      card => this.#visible(card)
    ).slice(0, this.#MAXIMUM_COLUMNS).map(
      /** @param {Element} card @returns {{x: number, y: number, width: number, height: number}} */
      card => this.#rectangle(card)
    );
    return {columns, cards};
  }

  /** @param {Element | null} grid @returns {number[]} */
  static #columns(grid) {
    if (grid === null) return [];
    return getComputedStyle(grid).gridTemplateColumns.split(/\s+/).map(Number.parseFloat).filter(Number.isFinite);
  }

  /** @param {Element} element @returns {boolean} */
  static #visible(element) {
    const rectangle = element.getBoundingClientRect();
    return [rectangle.width > 0, rectangle.height > 0].every(Boolean);
  }

  /** @param {Element} element @returns {{x: number, y: number, width: number, height: number}} */
  static #rectangle(element) {
    const value = element.getBoundingClientRect();
    return {x: value.x, y: value.y, width: value.width, height: value.height};
  }

  /** @returns {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} */
  static #interaction() {
    const text = document.body.dataset.qaBaseline;
    if (!text) return null;
    const baseline = JSON.parse(text);
    const scroll = Array.from(document.querySelectorAll(this.#SCROLL_SELECTOR)).find(
      /** @param {Element} element @returns {boolean} */
      element => /** @type {HTMLElement} */ (element).dataset.group === baseline.scroll?.id
    );
    return {baseline, current: JSON.parse(document.body.dataset.qaCurrent ?? '[]'), scroll: this.#scrollRecord(this.#element(scroll))};
  }

  /** @param {{horizontalOverflow: boolean, documentScroll: boolean, sidebarOverflow: boolean, tableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, historyLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, interaction: {baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null, sampleCount: number, live: string}} report @param {{check: string, width: number}} input @returns {void} */
  static #assertReport(report, input) {
    const value = report;
    this.#requirePage([!value.horizontalOverflow, !value.documentScroll, !value.sidebarOverflow].every(Boolean));
    this.#assertCheck(value, input);
  }

  /** @param {boolean} success @returns {void} */
  static #requirePage(success) {
    if (!success) throw new Error('Native QA found page or sidebar overflow.');
  }

  /** @param {{horizontalOverflow: boolean, documentScroll: boolean, sidebarOverflow: boolean, tableLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, historyLayout: {columns: number[], cards: {x: number, y: number, width: number, height: number}[]}, interaction: {baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null, sampleCount: number, live: string}} report @param {{check: string, width: number}} input @returns {void} */
  static #assertCheck(report, input) {
    switch (input.check) {
      case 'table': this.#requireGrid(report.tableLayout, this.#tableColumns(input.width)); return;
      case 'history': this.#requireGrid(report.historyLayout, this.#expectedColumns(input.width)); return;
      case 'activity-layout': this.#requireActivityGrid(this.#grid('.activity-grid', '.activity-card'), this.#tableColumns(input.width)); return;
      case 'zoom': this.#requireZoom(report.interaction); return;
      case 'polls': this.#requireZoom(report.interaction); this.#requireScroll(report.interaction); return;
      case 'hold': this.#requirePaused(report.sampleCount, report.live); return;
      default: return;
    }
  }

  /** @param {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} layout @param {number} expected @returns {void} */
  static #requireGrid(layout, expected) {
    const ready = this.#gridReady(layout, expected);
    this.#requireLayout(ready, expected, layout.columns.length);
  }

  /** @param {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} layout @param {number} expected @returns {void} */
  static #requireActivityGrid(layout, expected) {
    const ready = [layout.columns.length === expected, layout.cards.length > 0, this.#cardsFitColumns(layout)].every(Boolean);
    this.#requireLayout(ready, expected, layout.columns.length);
  }

  /** @param {{columns: number[], cards: {width: number}[]}} layout @returns {boolean} */
  static #cardsFitColumns(layout) {
    return layout.cards.every(card => Math.abs(card.width - layout.columns[0]) <= this.#SIZE_TOLERANCE);
  }

  /** @param {number} width @returns {number} */
  static #tableColumns(width) {
    if (width >= this.#FULL_WIDTH) return this.#MAXIMUM_COLUMNS;
    return this.#expectedColumns(width);
  }

  /** @param {number} width @returns {number} */
  static #expectedColumns(width) {
    if (width >= this.#WIDE_WIDTH) return this.#REQUIRED_GROUPS;
    return 1;
  }

  /** @param {{columns: number[], cards: {x: number, y: number, width: number, height: number}[]}} layout @param {number} expected @returns {boolean} */
  static #gridReady(layout, expected) {
    const sameRow = this.#sameRow(layout.cards);
    return [layout.columns.length === expected, layout.cards.length >= expected,
      this.#equalCardWidths(layout.cards), this.#expectedRow(expected, sameRow)].every(Boolean);
  }

  /** @param {{y: number}[]} cards @returns {boolean} */
  static #sameRow(cards) {
    if (cards.length < this.#REQUIRED_GROUPS) return false;
    return Math.abs(cards[0].y - cards[1].y) <= this.#SIZE_TOLERANCE;
  }

  /** @param {number} expected @param {boolean} sameRow @returns {boolean} */
  static #expectedRow(expected, sameRow) {
    if (expected === 1) return true;
    return sameRow;
  }

  /** @param {{width: number}[]} cards @returns {boolean} */
  static #equalCardWidths(cards) {
    return cards.every(
      /** @param {{width: number}} card @returns {boolean} */
      card => Math.abs(card.width - cards[0].width) <= this.#SIZE_TOLERANCE
    );
  }

  /** @param {boolean} ready @param {number} expected @param {number} actual @returns {void} */
  static #requireLayout(ready, expected, actual) {
    if (!ready) throw new Error('Native QA grid expected ' + expected + ' equal columns; measured ' + actual + '.');
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {void} */
  static #requireZoom(interaction) {
    const ready = this.#zoomReady(interaction);
    this.#requireZoomReady(ready);
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {boolean} */
  static #zoomReady(interaction) {
    if (interaction === null) return false;
    const baseline = interaction.baseline;
    return [baseline.before.length === this.#REQUIRED_GROUPS, baseline.after.length === this.#REQUIRED_GROUPS,
      baseline.before[0]?.id === baseline.after[0]?.id, baseline.before[1]?.id === baseline.after[1]?.id,
      baseline.before[0]?.scale !== baseline.after[0]?.scale, baseline.before[1]?.scale === baseline.after[1]?.scale,
      JSON.stringify(interaction.current) === JSON.stringify(baseline.after)].every(Boolean);
  }

  /** @param {boolean} ready @returns {void} */
  static #requireZoomReady(ready) {
    if (!ready) throw new Error('Native QA independent zoom or zoom preservation failed.');
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {void} */
  static #requireScroll(interaction) {
    const ready = this.#scrollReady(interaction);
    this.#requireScrollReady(ready);
  }

  /** @param {{baseline: {before: {id: string, scale: string}[], after: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null}, current: {id: string, scale: string}[], scroll: {id: string, top: number, left: number} | null} | null} interaction @returns {boolean} */
  static #scrollReady(interaction) {
    if (interaction === null) return false;
    return JSON.stringify(interaction.scroll) === JSON.stringify(interaction.baseline.scroll);
  }

  /** @param {boolean} ready @returns {void} */
  static #requireScrollReady(ready) {
    if (!ready) throw new Error('Native QA group pan changed across three measured updates.');
  }

  /** @param {number} samples @param {string} live @returns {void} */
  static #requirePaused(samples, live) {
    const ready = [samples === Number(document.body.dataset.qaPausedSamples), live.toLowerCase().includes('paused')].every(Boolean);
    this.#requirePauseReady(ready);
  }

  /** @param {boolean} ready @returns {void} */
  static #requirePauseReady(ready) {
    if (!ready) throw new Error('Native QA paused sample count changed during the hold.');
  }

  /** @returns {number} */
  static #samples() {
    return Number.parseFloat(this.#text(document.querySelector(this.#SAMPLE_SELECTOR)));
  }

  /** @returns {string} */
  static #live() {
    return this.#text(document.querySelector('.live-state'));
  }

  /** @param {Element | null} element @returns {string} */
  static #text(element) {
    if (element === null) return '';
    return String(element.textContent).trim();
  }

  /** @param {Element | null} canvas @returns {number | null} */
  static #points(canvas) {
    const text = this.#attribute(canvas, 'data-points');
    return this.#number(text);
  }

  /** @param {Element | null} element @param {string} name @returns {string | null} */
  static #attribute(element, name) {
    if (element === null) return null;
    return element.getAttribute(name);
  }

  /** @param {string | null} text @returns {number | null} */
  static #number(text) {
    const number = Number.parseFloat(String(text));
    if (!Number.isFinite(number)) return null;
    return number;
  }

  /** @param {Element | null} canvas @returns {string | null} */
  static #chartUnit(canvas) {
    const constructor = /** @type {Window & {Chart?: {getChart: (canvas: Element | null) => {options: {scales: {y: {title: {text: string}}}}} | undefined}}} */ (window).Chart;
    const chart = constructor?.getChart(canvas);
    return this.#unit(chart);
  }

  /** @param {{options: {scales: {y: {title: {text: string}}}}} | undefined} chart @returns {string | null} */
  static #unit(chart) {
    if (chart === undefined) return null;
    return chart.options.scales.y.title.text;
  }

  /** @returns {boolean} */
  static #sidebarOverflow() {
    const sidebar = document.querySelector('.sidebar');
    if (sidebar === null) return false;
    return sidebar.scrollHeight > sidebar.clientHeight;
  }
}
