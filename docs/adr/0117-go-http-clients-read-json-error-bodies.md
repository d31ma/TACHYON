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
