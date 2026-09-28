// Every annotation refuses a target that it does not belong on (ADR 0056), also in JavaScript checked with JSDoc.
class ServiceOnMethod {
  // @ts-expect-error Service belongs on a class, not on a method.
  @Service
  load() {}
}

class CachedOnField {
  // @ts-expect-error Cached belongs on a method, not on a field.
  @Cached
  value = 0;
}

class RequiredOnMethod {
  // @ts-expect-error Required belongs on a field, not on a method.
  @Required
  load() {}
}

// @ts-expect-error FromBody belongs on a parameter, not on a class.
@FromBody
class FromBodyOnClass {}

export const targets = [ServiceOnMethod, CachedOnField, RequiredOnMethod, FromBodyOnClass];
