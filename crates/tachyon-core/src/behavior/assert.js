
class Assert {
  static NOTHING_THROWN = 'Expected an error to be thrown, but nothing was thrown.'
  static MESSAGE_SEPARATOR = ' ==> '

  static equals(expected, actual, message) {
    const failure = `expected: <${Assert._show(expected)}> but was: <${Assert._show(actual)}>`
    Assert._check(Assert._same(expected, actual), failure, message)
  }

  static isTrue(condition, message) {
    const failure = `expected: <true> but was: <${Assert._show(condition)}>`
    Assert._check(condition === true, failure, message)
  }

  static throws(action, message) {
    const outcome = Assert._outcome(action)
    return Assert._thrown(outcome, message)
  }

  static _outcome(action) {
    try {
      return { failed: false, result: action() }
    } catch (error) {
      return { failed: true, error }
    }
  }

  static _thrown(outcome, message) {
    if (Assert._isPromise(outcome.result)) return Assert._rejected(outcome.result, message)
    return Assert._settled(outcome, message)
  }

  static _settled(outcome, message) {
    Assert._check(outcome.failed, Assert.NOTHING_THROWN, message)
    return outcome.error
  }

  static _rejected(promise, message) {
    return promise.then(() => Assert._fail(Assert.NOTHING_THROWN, message), (error) => error)
  }

  static _isPromise(value) {
    return typeof value?.then === 'function'
  }

  static _check(passed, failure, message) {
    if (!passed) Assert._fail(failure, message)
  }

  static _fail(failure, message) {
    throw new Error(Assert._text(failure, message))
  }

  static _text(failure, message) {
    if (message === undefined) return failure
    return `${message}${Assert.MESSAGE_SEPARATOR}${failure}`
  }

  static _same(expected, actual) {
    if (Object.is(expected, actual)) return true
    if (!Assert._isObject(expected) || !Assert._isObject(actual)) return false
    return Assert._sameShape(expected, actual)
  }

  static _isObject(value) {
    return typeof value === 'object' && value !== null
  }

  static _sameShape(expected, actual) {
    if (Object.getPrototypeOf(expected) !== Object.getPrototypeOf(actual)) return false
    if (expected instanceof Date) return expected.getTime() === actual.getTime()
    if (expected instanceof Map || expected instanceof Set) return Assert._same([...expected], [...actual])
    return Assert._sameKeys(expected, actual)
  }

  static _sameKeys(expected, actual) {
    const keys = Object.keys(expected)
    if (keys.length !== Object.keys(actual).length) return false
    return keys.every((key) => Assert._same(expected[key], actual[key]))
  }

  static _show(value) {
    if (typeof value === 'string') return value
    try {
      return JSON.stringify(value) ?? String(value)
    } catch {
      return String(value)
    }
  }
}
