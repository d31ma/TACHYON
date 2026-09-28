// Every annotation refuses a target that it does not belong on (ADR 0056).

class ServiceOnMethod {
  // @ts-expect-error Service belongs on a class, not on a method.
  @Service
  load(): void {}
}

class HelperOnMethod {
  // @ts-expect-error Helper belongs on a class, not on a method.
  @Helper
  load(): void {}
}

class ConverterOnMethod {
  // @ts-expect-error Converter belongs on a class, not on a method.
  @Converter
  load(): void {}
}

class SetupOnMethod {
  // @ts-expect-error Setup belongs on a class, not on a method.
  @Setup
  load(): void {}
}

class TestSetupOnMethod {
  // @ts-expect-error TestSetup belongs on a class, not on a method.
  @TestSetup
  load(): void {}
}

class CheckedOnMethod {
  // @ts-expect-error Checked belongs on a class, not on a method.
  @Checked
  load(): void {}
}

class FreshStartOnMethod {
  // @ts-expect-error FreshStart belongs on a class, not on a method.
  @FreshStart
  load(): void {}
}

class ErrorHandlerOnMethod {
  // @ts-expect-error ErrorHandler belongs on a class, not on a method.
  @ErrorHandler
  load(): void {}
}

class IdentityProviderOnMethod {
  // @ts-expect-error IdentityProvider belongs on a class, not on a method.
  @IdentityProvider
  load(): void {}
}

class PublicOnField {
  // @ts-expect-error Public belongs on a class or a method, not on a field.
  @Public
  value = 0;
}

class AllowWithoutRoles {
  // @ts-expect-error Allow takes at least one role type.
  @Allow()
  load(): void {}
}

class Admin {}

@RequestRoute("/orders")
@SignedIn
class GuardedController {
  @GetRoute
  @Public
  getOrders(): string[] {
    return [];
  }

  @DeleteRoute
  @Allow(Admin)
  deleteOrders(): string[] {
    return [];
  }
}

class RequestRouteOnMethod {
  // @ts-expect-error RequestRoute belongs on a class, not on a method.
  @RequestRoute
  load(): void {}
}

class HTTPClientOnMethod {
  // @ts-expect-error HTTPClient belongs on a class, not on a method.
  @HTTPClient
  load(): void {}
}

class TestOnMethod {
  // @ts-expect-error Test belongs on a class, not on a method.
  @Test(Object)
  load(): void {}
}

class UseProfilesOnMethod {
  // @ts-expect-error UseProfiles belongs on a class, not on a method.
  @UseProfiles("local")
  load(): void {}
}

class TestSettingsOnMethod {
  // @ts-expect-error TestSettings belongs on a class, not on a method.
  @TestSettings({"tac.size": 1})
  load(): void {}
}

class LifetimeOnMethod {
  // @ts-expect-error Lifetime belongs on a class, not on a method.
  @Lifetime("request")
  load(): void {}
}

class NeedsAllOnMethod {
  // @ts-expect-error NeedsAll belongs on a class, not on a method.
  @NeedsAll
  load(): void {}
}

class ScheduleOnMethod {
  // @ts-expect-error Schedule belongs on a class, not on a method.
  @Schedule({"fixedDelaySeconds": 30})
  load(): void {}
}

class CacheSettingsOnMethod {
  // @ts-expect-error CacheSettings belongs on a class, not on a method.
  @CacheSettings({"name": "catalog", "ttlMs": 1, "maxBytes": 1, "maxEntries": 1})
  load(): void {}
}

class EntityOnMethod {
  // @ts-expect-error Entity belongs on a class, not on a method.
  @Entity({"adapter": "data"})
  load(): void {}
}

class TableOnMethod {
  // @ts-expect-error Table belongs on a class, not on a method.
  @Table("items")
  load(): void {}
}

class DescriptionOnMethod {
  // @ts-expect-error Description belongs on a class, not on a method.
  @Description("Items")
  load(): void {}
}

class SettingsOnMethod {
  // @ts-expect-error Settings belongs on a class, not on a method.
  @Settings({"prefix": "YON_ITEMS_"})
  load(): void {}
}

class WhenProfileOnMethod {
  // @ts-expect-error WhenProfile belongs on a class, not on a method.
  @WhenProfile("local")
  load(): void {}
}

class WhenSettingOnMethod {
  // @ts-expect-error WhenSetting belongs on a class, not on a method.
  @WhenSetting({"name": "YON_ITEMS_ON", "havingValue": "true", "matchIfMissing": false})
  load(): void {}
}

class CachedOnField {
  // @ts-expect-error Cached belongs on a method, not on a field.
  @Cached
  value = 0;
}

class ClearCacheOnField {
  // @ts-expect-error ClearCache belongs on a method, not on a field.
  @ClearCache({"method": "list"})
  value = 0;
}

class UpdateCacheOnField {
  // @ts-expect-error UpdateCache belongs on a method, not on a field.
  @UpdateCache({"method": "list"})
  value = 0;
}

class RetryOnField {
  // @ts-expect-error Retry belongs on a method, not on a field.
  @Retry({"maxRetries": 2})
  value = 0;
}

class MaxAtOnceOnField {
  // @ts-expect-error MaxAtOnce belongs on a method, not on a field.
  @MaxAtOnce({"limit": 2})
  value = 0;
}

class AllOrNothingOnField {
  // @ts-expect-error AllOrNothing belongs on a method, not on a field.
  @AllOrNothing({"adapter": "data"})
  value = 0;
}

class GatewayOnField {
  // @ts-expect-error Gateway belongs on a method, not on a field.
  @Gateway({"command": "echo"})
  value = 0;
}

class ConvertOnField {
  // @ts-expect-error Convert belongs on a method, not on a field.
  @Convert({"fields": []})
  value = 0;
}

class QueryOnField {
  // @ts-expect-error Query belongs on a method, not on a field.
  @Query({"operation": "find"})
  value = 0;
}

class OnEventOnField {
  // @ts-expect-error OnEvent belongs on a method, not on a field.
  @OnEvent
  value = 0;
}

class OnStartOnField {
  // @ts-expect-error OnStart belongs on a method, not on a field.
  @OnStart
  value = 0;
}

class OnStopOnField {
  // @ts-expect-error OnStop belongs on a method, not on a field.
  @OnStop
  value = 0;
}

class OrderOnField {
  // @ts-expect-error Order belongs on a method, not on a field.
  @Order(1)
  value = 0;
}

class InBackgroundOnField {
  // @ts-expect-error InBackground belongs on a method, not on a field.
  @InBackground
  value = 0;
}

class HandlesErrorOnField {
  // @ts-expect-error HandlesError belongs on a method, not on a field.
  @HandlesError
  value = 0;
}

class GetRouteOnField {
  // @ts-expect-error GetRoute belongs on a method, not on a field.
  @GetRoute
  value = 0;
}

class PostRouteOnField {
  // @ts-expect-error PostRoute belongs on a method, not on a field.
  @PostRoute
  value = 0;
}

class PutRouteOnField {
  // @ts-expect-error PutRoute belongs on a method, not on a field.
  @PutRoute
  value = 0;
}

class PatchRouteOnField {
  // @ts-expect-error PatchRoute belongs on a method, not on a field.
  @PatchRoute
  value = 0;
}

class DeleteRouteOnField {
  // @ts-expect-error DeleteRoute belongs on a method, not on a field.
  @DeleteRoute
  value = 0;
}

class ResponseStatusOnField {
  // @ts-expect-error ResponseStatus belongs on a method, not on a field.
  @ResponseStatus(201)
  value = 0;
}

class GetRequestOnField {
  // @ts-expect-error GetRequest belongs on a method, not on a field.
  @GetRequest
  value = 0;
}

class PostRequestOnField {
  // @ts-expect-error PostRequest belongs on a method, not on a field.
  @PostRequest
  value = 0;
}

class PutRequestOnField {
  // @ts-expect-error PutRequest belongs on a method, not on a field.
  @PutRequest
  value = 0;
}

class PatchRequestOnField {
  // @ts-expect-error PatchRequest belongs on a method, not on a field.
  @PatchRequest
  value = 0;
}

class DeleteRequestOnField {
  // @ts-expect-error DeleteRequest belongs on a method, not on a field.
  @DeleteRequest
  value = 0;
}

class StreamRouteOnField {
  // @ts-expect-error StreamRoute belongs on a method, not on a field.
  @StreamRoute
  value = 0;
}

class StreamRequestOnField {
  // @ts-expect-error StreamRequest belongs on a method, not on a field.
  @StreamRequest
  value = 0;
}

class SocketRouteOnMethod {
  // @ts-expect-error SocketRoute belongs on a class, not on a method.
  @SocketRoute
  method() {}
}

class OnMessageOnField {
  // @ts-expect-error OnMessage belongs on a method, not on a field.
  @OnMessage
  value = 0;
}

class SocketRequestOnField {
  // @ts-expect-error SocketRequest belongs on a method, not on a field.
  @SocketRequest
  value = 0;
}

class FakeOnMethod {
  // @ts-expect-error Fake belongs on a field, not on a method.
  @Fake
  load(): void {}
}

class IdOnMethod {
  // @ts-expect-error Id belongs on a field, not on a method.
  @Id
  load(): void {}
}

class NamedOnMethod {
  // @ts-expect-error Named belongs on a field, not on a method.
  @Named("primary")
  load(): void {}
}

class LocalStateOnMethod {
  // @ts-expect-error LocalState belongs on a field, not on a method.
  @LocalState
  load(): void {}
}

class SessionStateOnMethod {
  // @ts-expect-error SessionState belongs on a field, not on a method.
  @SessionState
  load(): void {}
}

class KeptFields {
  @LocalState
  theme = 'light';
  @SessionState
  draft = '';
}

class SettingOnMethod {
  // @ts-expect-error Setting belongs on a field, not on a method.
  @Setting("${yon.size}")
  load(): void {}
}

class RequiredOnMethod {
  // @ts-expect-error Required belongs on a field, not on a method.
  @Required
  load(): void {}
}

class NotEmptyOnMethod {
  // @ts-expect-error NotEmpty belongs on a field, not on a method.
  @NotEmpty
  load(): void {}
}

class LengthOnMethod {
  // @ts-expect-error Length belongs on a field, not on a method.
  @Length({"min": 1, "max": 2})
  load(): void {}
}

class MinOnMethod {
  // @ts-expect-error Min belongs on a field, not on a method.
  @Min(1)
  load(): void {}
}

class MaxOnMethod {
  // @ts-expect-error Max belongs on a field, not on a method.
  @Max(2)
  load(): void {}
}

class NeedsOnMethod {
  // @ts-expect-error Needs belongs on a field, not on a method.
  @Needs
  load(): void {}
}

// @ts-expect-error CacheKey belongs on a parameter, not on a class.
@CacheKey
class CacheKeyOnClass {}

// @ts-expect-error Check belongs on a parameter, not on a class.
@Check
class CheckOnClass {}

// @ts-expect-error PageDefaults belongs on a parameter, not on a class.
@PageDefaults
class PageDefaultsOnClass {}

// @ts-expect-error FromPath belongs on a parameter, not on a class.
@FromPath
class FromPathOnClass {}

// @ts-expect-error FromQuery belongs on a parameter, not on a class.
@FromQuery
class FromQueryOnClass {}

// @ts-expect-error FromBody belongs on a parameter, not on a class.
@FromBody
class FromBodyOnClass {}

// @ts-expect-error FromHeader belongs on a parameter, not on a class.
@FromHeader
class FromHeaderOnClass {}

// @ts-expect-error FromCookie belongs on a parameter, not on a class.
@FromCookie
class FromCookieOnClass {}

// @ts-expect-error PathVariable belongs on a parameter, not on a class.
@PathVariable
class PathVariableOnClass {}

// @ts-expect-error RequestParam belongs on a parameter, not on a class.
@RequestParam
class RequestParamOnClass {}

// @ts-expect-error RequestBody belongs on a parameter, not on a class.
@RequestBody
class RequestBodyOnClass {}

// @ts-expect-error RequestHeader belongs on a parameter, not on a class.
@RequestHeader
class RequestHeaderOnClass {}

export const targets = [IdentityProviderOnMethod, PublicOnField, AllowWithoutRoles, GuardedController, ServiceOnMethod, HelperOnMethod, ConverterOnMethod, SetupOnMethod, TestSetupOnMethod, CheckedOnMethod, FreshStartOnMethod, ErrorHandlerOnMethod, RequestRouteOnMethod, HTTPClientOnMethod, TestOnMethod, UseProfilesOnMethod, TestSettingsOnMethod, LifetimeOnMethod, NeedsAllOnMethod, ScheduleOnMethod, CacheSettingsOnMethod, EntityOnMethod, TableOnMethod, DescriptionOnMethod, SettingsOnMethod, WhenProfileOnMethod, WhenSettingOnMethod, CachedOnField, ClearCacheOnField, UpdateCacheOnField, RetryOnField, MaxAtOnceOnField, AllOrNothingOnField, GatewayOnField, ConvertOnField, QueryOnField, OnEventOnField, OnStartOnField, OnStopOnField, OrderOnField, InBackgroundOnField, HandlesErrorOnField, GetRouteOnField, PostRouteOnField, PutRouteOnField, PatchRouteOnField, DeleteRouteOnField, ResponseStatusOnField, GetRequestOnField, PostRequestOnField, PutRequestOnField, PatchRequestOnField, DeleteRequestOnField, StreamRouteOnField, StreamRequestOnField, SocketRouteOnMethod, OnMessageOnField, SocketRequestOnField, FakeOnMethod, IdOnMethod, NamedOnMethod, SettingOnMethod, RequiredOnMethod, NotEmptyOnMethod, LengthOnMethod, MinOnMethod, MaxOnMethod, NeedsOnMethod, CacheKeyOnClass, CheckOnClass, PageDefaultsOnClass, FromPathOnClass, FromQueryOnClass, FromBodyOnClass, FromHeaderOnClass, FromCookieOnClass, PathVariableOnClass, RequestParamOnClass, RequestBodyOnClass, RequestHeaderOnClass];

class ConvertNeedsAPromise {
  // @ts-expect-error Convert writes an async method: it returns a Promise.
  @Convert({"fields": []})
  toSummary(value: unknown): unknown { return value; }
}

class QueryNeedsAPromise {
  // @ts-expect-error Query writes an async method: it returns a Promise.
  @Query({"operation": "find"})
  find(): unknown { return []; }
}

// @ts-expect-error WhenProfile names at least one profile.
@WhenProfile()
class ProfileWithoutName { static configure(): object { return {}; } }

// @ts-expect-error NeedsAll takes field names, not an options object.
@NeedsAll({"value": ["settings"]})
class NeedsAllWithOptions { settings!: string; }

// @ts-expect-error NeedsAll takes no field list; every field can be filled.
@NeedsAll('settings')
class NeedsAllWithFields { settings!: string; }

export const signatures = [LocalStateOnMethod, SessionStateOnMethod, KeptFields, ConvertNeedsAPromise, QueryNeedsAPromise, ProfileWithoutName, NeedsAllWithOptions, NeedsAllWithFields];
