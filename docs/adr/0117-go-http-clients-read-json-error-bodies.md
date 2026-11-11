# ADR 0117: Go HTTP Clients Read JSON Error Bodies

- Status: Accepted
- Date: 2026-11-10
- Amends: [ADR 0090](0090-compiled-dart-http-clients-await-the-browser-host.md)
  (its amendment of 2026-10-11 kept the body of an error answer out)
- Works with: [ADR 0022](0022-shared-behavior-adapters.md),
  [ADR 0103](0103-native-apps-reach-yon-through-a-declared-origin.md)

## Context

A Go Tac service calls the server through an `@HTTPClient`. When the server
answers with an error status, the browser transport (`tac-http.js`) answers
`{"error":{"code":"HTTP_STATUS","status":422}}`. Go reads the code with
`BehaviorErrorCode(err)` and the number with `BehaviorErrorStatus(err)`.
The body of the answer always stayed out.

A Yon `Check` that fails answers 422 with a problem body (RFC 9457):
`{"type":"about:blank","title":"Invalid input","status":422,"errors":[...]}`.
Other APIs answer 409 or 400 with a JSON reason. A page that wants to show
which field was wrong, or why a write was refused, could not read it.

A test had the same gap. A TestSetup adapter that returned
`&BehaviorError{Code: "HTTP_STATUS", Status: 409}` was rewritten to the code
`adapter`, so a test could not fake an error status at all.

## Decision

1. **The browser transport passes a JSON error body.** For an answer that is
   not `ok`, `tac-http.js` reads the body only when its media type (the part
   before `;`, in lower case) is `application/json` or
   `application/problem+json`. The body is read with the request's own answer
   limit (`maxResponseBytes`, at most 1 MiB), must be valid UTF-8 JSON, must
   pass the same value check as a success answer (depth 32, safe numbers) and
   must not be `null`. Then the answer is
   `{"error":{"code":"HTTP_STATUS","status":422,"body":<json>}}`.
   In every other case, and when the whole answer would be larger than 1 MiB,
   the answer has the code and the status only, as before. A failure while the
   body is read never changes the code; a cancel or a deadline during the read
   still answers `CANCELLED` or `DEADLINE`.
2. **Go reads it into a model.** `BehaviorError` has a new field
   `Body BehaviorValue`. The new function
   `BehaviorErrorBody(err error, model any) bool` encodes the body with
   `encoding/json` and decodes it into `model` (a pointer), with the model's
   `json` tags. It returns `false` when `err` is not a `BehaviorError`, has no
   body, or the body does not fit the model. This is a function, like
   `BehaviorErrorStatus` and `BehaviorErrorCode`, not a new marker: it needs
   no compiler change and works for every status.
3. **An adapter may answer with an error status.** A Setup or TestSetup
   adapter that returns
   `&BehaviorError{Code: "HTTP_STATUS", Status: 400..599, Body: ...}` now
   gives the call `HTTP_STATUS` with that status and that body. The body goes
   through the same 1 MiB value check. The adapter's own `Message` is replaced
   with "The HTTP request failed.", so adapter text never reaches the page.
   Every other adapter error, a status outside 400 to 599, or a body that
   fails the check, stays the code `adapter` as before. A cancel or a deadline
   still wins. This lives in `go_runtime.go`, so Yon Go services get it too.
4. **Other languages are unchanged.** Their browser clients read only
   `error.code` and ignore other members, so the extra `body` member is safe.
   They keep the code only. Adding the body to them is separate work.
5. **Native targets.** On macOS, Windows, Linux, iOS and Android a Go Tac
   service runs as WebAssembly in the platform web view and reaches Yon
   through the same `tac-http.js` (ADR 0103), so the body reaches Go there
   too. The native transports of Rust, Kotlin, Swift and C# native services
   stay status-only.

The body is the server's own answer to the page's own request, on the page
origin or the declared Yon origin; no new origin becomes reachable. The body
is data for the service, never a message shown by Tachyon.

## Not decided here

A binary request body for `PutRequest` and `PostRequest` stays out. The
browser transport and every language runtime send a JSON body only, and a Go
call is capped at 64 KiB. File uploads belong to the file capability.

## Consequences

- A Go page can show the fields a `Check` refused, or the reason of a 409.
- A Go test can fake an error status with a body through its TestSetup.
- An answer that is not JSON, or too large, still gives the status only.
- `scripts/browser-http-runtime-test.mjs` now runs in nextest
  (`browser_http_runtime_tests`), and the Go runtime contract and the Go
  `tac-http-verbs` unit test the new paths.

## Amendment (2026-11-11): every Tac service language

The user asked for the feature in every Tac service language, not only Go.
Point 4 above ("Other languages are unchanged") is replaced by this section.

Each language now keeps the `status` and the `body` of a failed call, reads
the body into a typed model with the same reader it uses for a success
answer, and lets a Setup or TestSetup adapter answer with `HTTP_STATUS`, a
status and a body. The rules of points 1 and 3 apply in every language.

| Language | Status number | Body as a typed model | A fake answers with |
| --- | --- | --- | --- |
| JavaScript | `Behaviors.errorStatus(error)` | `Behaviors.errorBody(error)`, a copy or `null` | `throw Behaviors.httpError(409, body)` |
| TypeScript | `Behaviors.errorStatus(error)` | `Behaviors.errorBody<Problem>(error)`, a copy or `null` | `throw Behaviors.httpError(409, body)` |
| Go | `BehaviorErrorStatus(err)` | `BehaviorErrorBody(err, &model)`, `bool` | `&BehaviorError{Code: "HTTP_STATUS", Status: 409, Body: body}` |
| Rust | `error.status` | `error.body_as::<Problem>()`, `Option<Problem>` | `Err(BehaviorError::http_status(409, Some(body)))` |
| C# | `error.Status` | `error.TryReadBody(out Problem model)`, `bool` | `throw new BehaviorError("HTTP_STATUS", 409, body, message)` |
| F# | `error.Status` | `TacMappingModels.errorBodyProblem error`, `Problem option` | `raise (BehaviorError("HTTP_STATUS", 409, body, message))` |
| Swift | `Behaviors.errorStatus(error)` | `Behaviors.errorBody(error, as: Problem.self)`, `Problem?` | `throw BehaviorError("HTTP_STATUS", status: 409, body: body)` |
| Kotlin | `error.status` | `Behaviors.errorBody<Problem>(error)`, `Problem?` | `throw Behaviors.Failure("HTTP_STATUS", null, message, 409, body)` |
| Java | `Behaviors.errorStatus(error)` | `Behaviors.errorBody(error, TacModels::readProblem)`, `Optional<Problem>` | `throw new Behaviors.Failure("HTTP_STATUS", null, message, 409, body)` |
| Dart | `BehaviorErrors.status(error)` | `BehaviorErrors.body(error, tachyonReadModelProblem)`, `Problem?` | `throw BehaviorError.http(409, body)` |
| Ruby | `BehaviorError.status_of(error)` | `BehaviorError.body_of(error, Problem)`, the model or `nil` | `raise BehaviorError.new("HTTP_STATUS", message, nil, 409, body)` |
| Python | `BehaviorError.status_of(error)` | `BehaviorError.body_of(error, Problem)`, the model or `None` | `raise BehaviorError('HTTP_STATUS', message, None, 409, body)` |
| Lua | `BehaviorError.status_of(failure)` | `BehaviorError.body_of(failure, Problem)`, the model or `nil` | `BehaviorError.raise("HTTP_STATUS", message, nil, 409, body)` |
| C++ | `error.statusCode()` | `error.readBodyFromJson<Problem>()`, `std::optional<Problem>` | `throw HttpRequestException(409, body)` |

- The body comes only from a JSON error answer (`application/json` or
  `application/problem+json`) that fits the answer limit and passes the value
  check. Otherwise the error has the status only.
- A fake's status must be 400 to 599 and its body must pass the same value
  check. Otherwise the call gets the language's usual adapter failure code
  (`adapter`, `ADAPTER_FAILED` or `http-adapter`).
- The fake's own message never reaches the caller: the caller gets
  "The HTTP request failed.".
- Native Rust and native C# services keep the status only. Their native HTTP
  layers carry no media type, so the JSON check cannot run there.
- JavaScript, TypeScript and Lua do not check the body's field types, as with
  a success answer.
- JavaScript and TypeScript services send their requests through
  `client_runtime.js`, not `tac-http.js`, so `client_runtime.js` applies the
  same rule as point 1.
- Swift on Apple native targets and Kotlin on Android read the body in the
  native transport too, for the same two media types.
- In JavaScript, TypeScript and Go, `Retry` still retries an `HTTP_STATUS`
  error from an adapter, as it did when that error was an adapter error.

Tests: each language's `tac-http-verbs` unit has a `testErrorBody` case (F#
in `tac_fsharp_http.rs`, Java in `java_http_error_tests.rs`), and the
runtime tests of each language check the adapter rule.
