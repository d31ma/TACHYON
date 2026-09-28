

class Assert:
    NOTHING_THROWN = 'Expected an error to be thrown, but nothing was thrown.'
    MESSAGE_SEPARATOR = ' ==> '

    @staticmethod
    def equals(expected, actual, message=None):
        failure = f'expected: <{expected!r}> but was: <{actual!r}>'
        Assert._check(expected == actual, failure, message)

    @staticmethod
    def is_true(condition, message=None):
        failure = f'expected: <True> but was: <{condition!r}>'
        Assert._check(condition is True, failure, message)

    @staticmethod
    def throws(action, message=None):
        outcome = Assert._outcome(action)
        return Assert._thrown(outcome, message)

    @staticmethod
    def _outcome(action):
        try:
            return (False, action())
        except Exception as error:
            return (True, error)

    @staticmethod
    def _thrown(outcome, message):
        failed, value = outcome
        if failed:
            return value
        if _behavior_inspect.isawaitable(value):
            return Assert._raised(value, message)
        return Assert._fail(Assert.NOTHING_THROWN, message)

    @staticmethod
    async def _raised(awaitable, message):
        try:
            await awaitable
        except Exception as error:
            return error
        return Assert._fail(Assert.NOTHING_THROWN, message)

    @staticmethod
    def _check(passed, failure, message):
        if not passed:
            Assert._fail(failure, message)

    @staticmethod
    def _fail(failure, message):
        text = failure if message is None else f'{message}{Assert.MESSAGE_SEPARATOR}{failure}'
        raise AssertionError(text)
