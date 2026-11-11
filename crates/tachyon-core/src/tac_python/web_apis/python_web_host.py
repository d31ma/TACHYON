import builtins as _tac_web_builtins
import json as _tac_web_json


class _TacPythonWebHost:
    @classmethod
    def call(cls, api: str, call: str, arguments: tuple = ()) -> object:
        request = {'api': api, 'call': call, 'args': list(arguments)}
        answer = _tac_web_json.loads(cls._ask(_tac_web_json.dumps(request, allow_nan=False)))
        return cls._value(answer)

    @staticmethod
    def _ask(text: str) -> str:
        host = getattr(_tac_web_builtins, 'TacWeb', None)
        if callable(host):
            return str(host(text))
        return BrowserFakes.reply(text)

    @staticmethod
    def _value(answer: dict) -> object:
        if type(answer.get('error')) is str:
            raise RuntimeError(answer['error'])
        return answer.get('value')
