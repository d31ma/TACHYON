import asyncio as _tac_asyncio
import json as _tac_json

class _TacPythonTasks:
    MAX_TASKS = 256
    MAX_ANSWER = 1048576
    FAILED = 'Python Tac operation failed.'
    CANCELLED = 'Python Tac task owner was disposed.'

    def __init__(self, codec):
        self.codec = codec
        self.running = {}
        self.next_id = 0
        self.done = None

    def require_capacity(self):
        if len(self.running) >= self.MAX_TASKS:
            raise ValueError('Python Tac task capacity was exceeded.')

    def start(self, owner, returns, awaitable):
        self.next_id += 1
        task_id = self.next_id
        task = _tac_asyncio.ensure_future(awaitable)
        self.running[task_id] = (owner, task)
        task.add_done_callback(lambda finished: self._finish(task_id, owner, returns, finished))
        return task_id

    def cancel_owner(self, owner):
        for task_owner, task in list(self.running.values()):
            if task_owner is owner:
                task.cancel()

    def cancel_all(self):
        for _, task in list(self.running.values()):
            task.cancel()

    def _finish(self, task_id, owner, returns, task):
        self.running.pop(task_id, None)
        answer = self._answer(owner, returns, task)
        if self.done is not None:
            self.done(task_id, answer)

    def _answer(self, owner, returns, task):
        if task.cancelled() or owner.disposed:
            return self._cancelled()
        if task.exception() is not None:
            return self._failed(task.exception())
        return self._value(returns, task.result())

    def _value(self, returns, value):
        try:
            result = self.codec.result(returns, value)
            answer = _tac_json.dumps({'value': self.codec.plain(result)}, allow_nan=False)
        except Exception:
            return self._failed()
        if len(answer.encode('utf-8')) > self.MAX_ANSWER:
            return self._failed()
        return answer

    def _failed(self, error=None):
        if type(error).__name__ != 'BehaviorError':
            return _tac_json.dumps({'error': self.FAILED})
        return _tac_json.dumps({'error': str(error)[:1024], 'code': str(getattr(error, 'code', 'behavior'))[:64]})

    def _cancelled(self):
        return _tac_json.dumps({'error': self.CANCELLED, 'code': 'CANCELLED'})
