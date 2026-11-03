class _TacPythonPublisher:
    def __init__(self, runtime, owner):
        self._runtime = runtime
        self._owner = owner

    def publish_event(self, value, retain=False):
        self._runtime.publish(self._owner, type(value).__name__, value, retain is True)
        return value
