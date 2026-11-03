const __yonEventCopy = value => {
  let nodes = 0, characters = 0;
  const visit = (item, depth = 0) => {
    if (depth > 32 || ++nodes > 8192) throw new RangeError('Local event depth exceeded');
    if (typeof item === 'string') { characters += item.length; if (characters > 65536) throw new RangeError('Local event payload exceeded'); return; }
    if (item === null || typeof item === 'boolean') return;
    if (typeof item === 'number' && Number.isFinite(item)) return;
    if (typeof item !== 'object' || (!Array.isArray(item) && ![null,Object.prototype].includes(Object.getPrototypeOf(item)))) throw new TypeError('Local events require finite JSON');
    const keys = Reflect.ownKeys(item);
    const array = Array.isArray(item);
    if (keys.length > 8192 || (array && (item.length > 8192 || keys.length !== item.length + 1))) throw new TypeError('Local events require dense JSON arrays');
    for (const key of keys) {
      if (array && key === 'length') continue;
      if (typeof key !== 'string' || (array && (!/^(0|[1-9][0-9]*)$/.test(key) || Number(key) >= item.length))) throw new TypeError('Local events require JSON keys');
      characters += key.length;
      if (characters > 65536) throw new RangeError('Local event payload exceeded');
      const descriptor = Object.getOwnPropertyDescriptor(item,key);
      if (!descriptor || !('value' in descriptor) || !descriptor.enumerable) throw new TypeError('Local events require data properties');
      visit(descriptor.value, depth + 1);
    }
  };
  visit(value);
  const encoded = JSON.stringify(value);
  if (new TextEncoder().encode(encoded).byteLength > 65536) throw new RangeError('Local event payload exceeded');
  return encoded;
};
function __yonEventPublish(name, value, retain) {
  if (value && typeof value.then === 'function') return Promise.resolve(value).then(result => __yonEventPublish(name, result, retain));
  try {
  const scope = __YonEvents.active;
  if (!scope || scope.closed) throw new Error('Local event owner is disposed');
  const encoded = __yonEventCopy(value);
  if (scope.queue.length >= 128) throw new RangeError('Local event queue exceeded');
  if (retain) scope.retained.set(name, encoded);
  scope.queue.push([name, encoded]);
  if (__YonEvents.topics.has(name)) scope.outbound.push([name, encoded]);
  } catch { process.stderr.write('Yon local event publication failed.\n'); }
  return value;
}
class ApplicationEventPublisher {
  publishEvent(name, value, retain = false) { __yonEventPublish(name, value, retain === true); return value; }
}
function __yonEventFits(models, shape, value) {
  if (shape.startsWith('?')) return value === null || __yonEventFits(models, shape.slice(1), value);
  if (shape.startsWith('[]')) return Array.isArray(value) && value.every(item => __yonEventFits(models, shape.slice(2), item));
  if (shape === 'any') return true;
  if (shape === 'integer') return Number.isInteger(value);
  if (shape === 'string' || shape === 'number' || shape === 'boolean') return typeof value === shape;
  const model = models[shape];
  if (!model || value === null || typeof value !== 'object' || Array.isArray(value)) return false;
  const names = Object.keys(model.fields);
  if (!model.open && Object.keys(value).some(key => !names.includes(key))) return false;
  for (const name of names) {
    const field = model.fields[name];
    const required = field.startsWith('!');
    if (!Object.hasOwn(value, name)) { if (required) return false; continue; }
    if (!__yonEventFits(models, required ? field.slice(1) : field, value[name])) return false;
  }
  return true;
}
function __yonEventMatches(matcher, value, event) {
  const operand = (node) => {
    if (node.k === 'lit') return node.v;
    if (node.k === 'event') return event;
    let current = value;
    for (const segment of node.path || []) {
      if (current === null || typeof current !== 'object') return undefined;
      current = current[segment];
    }
    return current;
  };
  const test = (node) => {
    switch (node.k) {
      case 'and': return test(node.l) && test(node.r);
      case 'or': return test(node.l) || test(node.r);
      case 'not': return !test(node.e);
      case 'truthy': return Boolean(operand(node.e));
      case 'cmp': {
        const left = operand(node.l), right = operand(node.r);
        if (node.op === 'eq') return left === right;
        if (node.op === 'ne') return left !== right;
        if (typeof left !== typeof right || (typeof left !== 'number' && typeof left !== 'string')) return false;
        if (node.op === 'lt') return left < right;
        if (node.op === 'le') return left <= right;
        if (node.op === 'gt') return left > right;
        return left >= right;
      }
      default: return false;
    }
  };
  try { return test(matcher); } catch { return false; }
}
class __YonEvents {
  static active = null;
  static topics = new Set();
  constructor(only = null) {
    if (__YonEvents.active) throw new Error('Nested local event invocation');
    this.queue = []; this.retained = new Map(); this.listeners = []; this.destroyers = []; this.closed = false; this.deliveries = 0;
    this.outbound = []; this.only = only;
    __YonEvents.active = this;
  }
  listen(options, callback, replay, key = '') {
    if (this.only && !this.only.has(key)) return;
    const names = typeof options === 'string' ? [options] : options.events;
    for (const name of names) {
      if (this.listeners.length >= 1024) throw new RangeError('Local listener capacity exceeded');
      this.listeners.push({ name, callback, order: options.order || 0, condition: options.condition || null, asynchronous: options.async === true, models: options.models || null, returns: options.returns || null, sequence: this.listeners.length });
      if (replay && this.retained.has(name)) this.queue.push([name, this.retained.get(name)]);
    }
    this.listeners.sort((left, right) => left.order - right.order || left.sequence - right.sequence);
  }
  __republish(result, returns) {
    if (!returns) return;
    for (const item of (Array.isArray(result) ? result : [result]).slice(0, 32)) {
      if (item !== null && item !== undefined) __yonEventPublish(returns, item, false);
    }
  }
  async drain() {
    const deferred = [];
    while (this.queue.length) {
      const [name, encoded] = this.queue.shift();
      if (++this.deliveries > 4096) { this.queue.length = 0; process.stderr.write('Yon local event delivery bound exceeded.\n'); return; }
      for (const listener of this.listeners) if (listener.name === name) {
        const value = JSON.parse(encoded);
        if (listener.models && !__yonEventFits(listener.models, name, value)) { process.stderr.write('Yon local event payload does not fit the listener model.\n'); continue; }
        if (listener.condition && !__yonEventMatches(listener.condition, value, name)) continue;
        if (listener.asynchronous) { deferred.push([listener, value]); continue; }
        try { this.__republish(await listener.callback(value), listener.returns); }
        catch { process.stderr.write('Yon local event listener failed.\n'); }
      }
      for (const [listener, value] of deferred.splice(0)) {
        try { await listener.callback(value); }
        catch { process.stderr.write('Yon local event listener failed.\n'); }
      }
    }
    for (const [name, encoded] of this.outbound.splice(0)) {
      try { await globalThis.YonTopic.publish(name, JSON.parse(encoded)); }
      catch { process.stderr.write('Yon topic publication failed.\n'); }
    }
  }
  async close() {
    if (this.closed) return;
    this.closed = true; this.queue.length = 0; this.listeners.length = 0;
    for (const destroy of this.destroyers.reverse()) {
      try { await destroy(); } catch { process.stderr.write('Yon local lifecycle cleanup failed.\n'); }
    }
    this.retained.clear(); __YonEvents.active = null;
  }
}
