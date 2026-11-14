
class BrokenRule {
  readonly field: string
  readonly rule: string

  constructor(value: any) {
    this.field = String(value?.field ?? '')
    this.rule = String(value?.rule ?? '')
  }
}

class YonBuiltInError extends Error {
  readonly status: number
  readonly reference: string

  constructor(name: string, status: number, value: any) {
    super(name)
    this.name = name
    this.status = status
    this.reference = String(value?.reference ?? '')
  }
}

class NotSignedIn extends YonBuiltInError {
  constructor(value: any) { super('NotSignedIn', 401, value) }
}

class RoleMissing extends YonBuiltInError {
  constructor(value: any) { super('RoleMissing', 403, value) }
}

class IdentityCheckFailed extends YonBuiltInError {
  constructor(value: any) { super('IdentityCheckFailed', 500, value) }
}

class RequestRejected extends YonBuiltInError {
  readonly part: string
  readonly field: string

  constructor(value: any) {
    super('RequestRejected', 400, value)
    this.part = String(value?.part ?? '')
    this.field = String(value?.field ?? '')
    this.message = String(value?.message ?? '')
  }
}

class BodyTooLarge extends YonBuiltInError {
  readonly limitBytes: number

  constructor(value: any) {
    super('BodyTooLarge', 413, value)
    this.limitBytes = Number(value?.limitBytes ?? 0)
  }
}

class CheckRejected extends YonBuiltInError {
  readonly violations: BrokenRule[]
  readonly truncated: boolean

  constructor(value: any) {
    super('CheckRejected', 422, value)
    this.violations = (value?.violations ?? []).map((entry: any) => new BrokenRule(entry))
    this.truncated = value?.truncated === true
  }
}

class UnexpectedError extends YonBuiltInError {
  constructor(value: any) { super('UnexpectedError', 500, value) }
}

function __yon_built_in_error__(value: any): YonBuiltInError {
  const types: Record<string, new (value: any) => YonBuiltInError> = { NotSignedIn, RoleMissing, IdentityCheckFailed, RequestRejected, BodyTooLarge, CheckRejected, UnexpectedError }
  const type = Object.hasOwn(types, value?.type) ? types[value.type] : undefined
  if (!type) throw new TypeError('Unknown built-in error.')
  return new type(value)
}
