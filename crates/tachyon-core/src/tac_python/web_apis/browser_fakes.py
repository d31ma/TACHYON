import json as _tac_web_json


class BrowserFakes:
    MISSING = 'This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.answer in the test unit.'
    answers = {}

    @classmethod
    def answer(cls, api: str, call: str, value: object) -> None:
        cls.answers[api + '.' + call] = value

    @classmethod
    def reply(cls, text: str) -> str:
        request = _tac_web_json.loads(text)
        return _tac_web_json.dumps(cls._answer_for(request['api'] + '.' + request['call']))

    @classmethod
    def _answer_for(cls, key: str) -> dict:
        if key not in cls.answers:
            return {'error': cls.MISSING}
        return {'value': cls.answers[key]}
