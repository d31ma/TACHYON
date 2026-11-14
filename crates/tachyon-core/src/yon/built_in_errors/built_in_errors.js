
class BrokenRule {
  constructor(value) {
    this.field = String(value?.field ?? '')
    this.rule = String(value?.rule ?? '')
  }
}

class YonBuiltInError extends Error {
  constructor(name, status, value) {
    super(name)
    this.name = name
    this.status = status
    this.reference = String(value?.reference ?? '')
  }
}

class NotSignedIn extends YonBuiltInError {
  constructor(value) { super('NotSignedIn', 401, value) }
}

class RoleMissing extends YonBuiltInError {
  constructor(value) { super('RoleMissing', 403, value) }
}

class IdentityCheckFailed extends YonBuiltInError {
  constructor(value) { super('IdentityCheckFailed', 500, value) }
}

class RequestRejected extends YonBuiltInError {
  constructor(value) {
    super('RequestRejected', 400, value)
    this.part = String(value?.part ?? '')
    this.field = String(value?.field ?? '')
    this.message = String(value?.message ?? '')
  }
}

class BodyTooLarge extends YonBuiltInError {
  constructor(value) {
    super('BodyTooLarge', 413, value)
    this.limitBytes = Number(value?.limitBytes ?? 0)
  }
}

class CheckRejected extends YonBuiltInError {
  constructor(value) {
    super('CheckRejected', 422, value)
    this.violations = (value?.violations ?? []).map((entry) => new BrokenRule(entry))
    this.truncated = value?.truncated === true
  }
}

class UnexpectedError extends YonBuiltInError {
  constructor(value) { super('UnexpectedError', 500, value) }
}

function __yon_built_in_error__(value) {
  const types = { NotSignedIn, RoleMissing, IdentityCheckFailed, RequestRejected, BodyTooLarge, CheckRejected, UnexpectedError }
  const type = Object.hasOwn(types, value?.type) ? types[value.type] : undefined
  if (!type) throw new TypeError('Unknown built-in error.')
  return new type(value)
}
