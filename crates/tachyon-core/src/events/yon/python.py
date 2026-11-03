import json as _event_json
import inspect as _event_inspect
import sys as _event_sys

def _yon_event_publish(name, value, retain):
    try:
        _yon_event_enqueue(name, value, retain)
    except Exception:
        _event_sys.stderr.write('Yon local event publication failed.\n')
    return value

def _yon_event_enqueue(name, value, retain):
    scope = _YonEvents.active
    if scope is None or scope.closed:
        raise RuntimeError('Local event owner is disposed')
    encoded = _event_json.dumps(value, allow_nan=False, separators=(',', ':'))
    too_large = len(encoded.encode('utf-8')) > _YonEvents.MAX_EVENT_BYTES
    if too_large or len(scope.queue) >= _YonEvents.MAX_QUEUED_EVENTS:
        raise ValueError('Local event capacity exceeded')
    if retain:
        scope.retained[name] = encoded
    scope.queue.append((name, encoded))
    if name in _YonEvents.topics:
        scope.outbound.append((name, encoded))
    return value

def _yon_event_publish_model(value, retain):
    try:
        if not YonModels.is_record(type(value)):
            raise TypeError('Local events publish model values')
        _yon_event_enqueue(type(value).__name__, YonModels.plain(value), retain)
    except Exception:
        _event_sys.stderr.write('Yon local event publication failed.\n')
    return value

class ApplicationEventPublisher:
    def publishEvent(self, value, retain=False):
        return _yon_event_publish_model(value, retain is True)
    publish_event = publishEvent

def _yon_event_matches(matcher, value, event):
    try:
        return _YonEvents._test(matcher, value, event)
    except Exception:
        return False

class _YonEvents:
    active = None
    topics = frozenset()
    MAX_EVENT_BYTES = 65536
    MAX_QUEUED_EVENTS = 128
    MAX_LISTENERS = 1024
    MAX_DELIVERIES = 4096
    MAX_REPUBLISHED_EVENTS = 32

    @staticmethod
    def _operand(node, value, event):
        if node['k'] == 'lit':
            return node.get('v')
        if node['k'] == 'event':
            return event
        current = value
        for segment in node.get('path', []):
            if isinstance(current, dict):
                current = current.get(segment)
            elif isinstance(current, list) and isinstance(segment, int) and 0 <= segment < len(current):
                current = current[segment]
            else:
                return None
        return current
    @staticmethod
    def _is_number(item):
        return isinstance(item, (int, float)) and not isinstance(item, bool)
    @staticmethod
    def _equal(left, right):
        return left == right and type(left) is type(right)
    @staticmethod
    def _compare(node, value, event):
        left = _YonEvents._operand(node['l'], value, event)
        right = _YonEvents._operand(node['r'], value, event)
        if node['op'] == 'eq':
            return _YonEvents._equal(left, right)
        if node['op'] == 'ne':
            return not _YonEvents._equal(left, right)
        both_numbers = _YonEvents._is_number(left) and _YonEvents._is_number(right)
        both_strings = isinstance(left, str) and isinstance(right, str)
        if not both_numbers and not both_strings:
            return False
        if node['op'] == 'lt':
            return left < right
        if node['op'] == 'le':
            return left <= right
        if node['op'] == 'gt':
            return left > right
        return left >= right
    @staticmethod
    def _test(node, value, event):
        kind = node['k']
        if kind == 'and':
            return _YonEvents._test(node['l'], value, event) and _YonEvents._test(node['r'], value, event)
        if kind == 'or':
            return _YonEvents._test(node['l'], value, event) or _YonEvents._test(node['r'], value, event)
        if kind == 'not':
            return not _YonEvents._test(node['e'], value, event)
        if kind == 'truthy':
            return bool(_YonEvents._operand(node['e'], value, event))
        if kind == 'cmp':
            return _YonEvents._compare(node, value, event)
        return False
    @staticmethod
    def _fits(models, shape, value):
        if shape.startswith('?'):
            return value is None or _YonEvents._fits(models, shape[1:], value)
        if shape.startswith('[]'):
            if not isinstance(value, list):
                return False
            return all(_YonEvents._fits(models, shape[2:], item) for item in value)
        if shape == 'any':
            return True
        if shape == 'string':
            return isinstance(value, str)
        if shape == 'integer':
            return isinstance(value, int) and not isinstance(value, bool)
        if shape == 'number':
            return _YonEvents._is_number(value)
        if shape == 'boolean':
            return isinstance(value, bool)
        model = models.get(shape)
        if model is None or not isinstance(value, dict):
            return False
        fields = model['fields']
        if not model['open'] and any(key not in fields for key in value):
            return False
        for name, field in fields.items():
            required = field.startswith('!')
            if name not in value:
                if required:
                    return False
                continue
            if not _YonEvents._fits(models, field[1:] if required else field, value[name]):
                return False
        return True
    @staticmethod
    def _listener_rank(listener):
        return (listener['order'], listener['sequence'])
    @staticmethod
    async def _call(listener, value):
        result = listener['callback'](value)
        if _event_inspect.isawaitable(result):
            result = await result
        return result
    def __init__(self, only=None):
        if _YonEvents.active is not None:
            raise RuntimeError('Nested local event invocation')
        self.only = only
        self.outbound = []
        self.queue = []
        self.retained = {}
        self.listeners = []
        self.destroyers = []
        self.closed = False
        self.deliveries = 0
        _YonEvents.active = self
    def listen(self, options, callback, replay, key=''):
        if self.only is not None and key not in self.only:
            return
        if isinstance(options, str):
            names = [options]
        else:
            names = options['events']
        order = 0
        condition = None
        asynchronous = False
        models = None
        returns = None
        if isinstance(options, dict):
            order = options.get('order', 0)
            condition = options.get('condition')
            asynchronous = bool(options.get('async'))
            models = options.get('models')
            returns = options.get('returns')
        for name in names:
            if len(self.listeners) >= _YonEvents.MAX_LISTENERS:
                raise ValueError('Local listener capacity exceeded')
            self.listeners.append({'name': name, 'callback': callback, 'order': order,
                                   'condition': condition, 'asynchronous': asynchronous,
                                   'models': models, 'returns': returns,
                                   'sequence': len(self.listeners)})
            if replay and name in self.retained:
                self.queue.append((name, self.retained[name]))
        self.listeners.sort(key=_YonEvents._listener_rank)
    def _republish(self, result, returns):
        if returns is None:
            return
        if isinstance(result, list):
            items = result
        else:
            items = [result]
        for item in items[:_YonEvents.MAX_REPUBLISHED_EVENTS]:
            if item is not None:
                _yon_event_publish(returns, YonModels.plain(item), False)
    async def _deliver_now(self, listener, value):
        try:
            result = await _YonEvents._call(listener, value)
            self._republish(result, listener['returns'])
        except Exception:
            _event_sys.stderr.write('Yon local event listener failed.\n')
    async def _deliver_later(self, listener, value):
        try:
            await _YonEvents._call(listener, value)
        except Exception:
            _event_sys.stderr.write('Yon local event listener failed.\n')
    async def _deliver(self, name, encoded):
        deferred = []
        for listener in self.listeners:
            if listener['name'] != name:
                continue
            value = _event_json.loads(encoded)
            models = listener['models']
            if models is not None and not _YonEvents._fits(models, name, value):
                _event_sys.stderr.write('Yon local event payload does not fit the listener model.\n')
                continue
            condition = listener['condition']
            if condition is not None and not _yon_event_matches(condition, value, name):
                continue
            if listener['asynchronous']:
                deferred.append((listener, value))
                continue
            await self._deliver_now(listener, value)
        return deferred
    async def drain(self):
        while self.queue:
            name, encoded = self.queue.pop(0)
            self.deliveries += 1
            if self.deliveries > _YonEvents.MAX_DELIVERIES:
                self.queue.clear()
                _event_sys.stderr.write('Yon local event delivery bound exceeded.\n')
                return
            deferred = await self._deliver(name, encoded)
            for listener, value in deferred:
                await self._deliver_later(listener, value)
        while self.outbound:
            name, encoded = self.outbound.pop(0)
            try:
                YonTopic.publish(name, _event_json.loads(encoded))
            except Exception:
                _event_sys.stderr.write('Yon topic publication failed.\n')
    async def _run_destroyers(self):
        for destroy in reversed(self.destroyers):
            try:
                result = destroy()
                if _event_inspect.isawaitable(result):
                    await result
            except Exception:
                _event_sys.stderr.write('Yon local lifecycle cleanup failed.\n')
    async def close(self):
        if self.closed:
            return
        self.closed = True
        self.queue.clear()
        self.listeners.clear()
        await self._run_destroyers()
        self.retained.clear()
        _YonEvents.active = None
