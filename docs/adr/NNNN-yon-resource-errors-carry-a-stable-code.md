# ADR NNNN: Yon Resource Errors Carry a Stable Code

- Status: Accepted
- Date: 2026-11-21
- Number: given by the TACHYON Build Coordinator at merge time, in merge order
- Builds on: [ADR 0027](0027-scoped-discovery-and-managed-resources.md)
  (managed resources) and
  [ADR 0128](0128-a-busy-fylo-root-waits-and-store-failures-are-logged.md)
  (store failures are logged; one store deadline per answer)
- Requested by: Mesail slice K38 (one FYLO transaction per user action)

## Context

Every failed managed-resource call reached the handler as one bare error with
the text "Yon managed resource operation failed." (Go `YonResourceError{}`,
Java `IllegalStateException`, and so on). The resource endpoint answered every
failure with the same body, and every storage failure was the diagnostic
`TY2160` with Tachyon's own text.

So an application could not tell a failure worth starting again (a spent
store budget, a busy root, a write conflict) from a real failure (a missing
document, a size limit, a refused transaction). Mesail's K38 needs that
difference: after one FYLO transaction per user action, "nothing was written,
try again" and "this action can never work" need different answers.

The text cannot carry more detail. Tachyon's diagnostic messages and FYLO's
own messages can name root paths, process IDs and SQL, which must never reach
a handler (ADR 0128).

## Decision

1. **A closed set of codes.** Every resource error carries one code from this
   set. A code is a fixed lowercase word; it never holds a path, a root name,
   an identifier, SQL or a message from the database.

   | Code | Meaning | Was anything written? |
   | --- | --- | --- |
   | `budget_spent` | the answer's store-time budget (ADR 0128, decision 6) or the call's own deadline ran out | maybe: writes may have committed |
   | `busy` | the store is busy now | no |
   | `conflict` | a write lost to another write (a duplicate key) | no |
   | `not_found` | the document or collection does not exist | no |
   | `too_large` | a size limit was passed | no |
   | `transaction_refused` | the transaction cannot be started or used | no, by that transaction |
   | `unavailable` | the store could not be reached | no |
   | `failed` | anything else | unknown |

   `failed` is also the answer for any code a prelude does not know, so a
   newer `ty` with a new code never breaks an older handler.

2. **Where each code comes from.** One table in the code,
   `ResourceErrorCode::FYLO_CODES`, maps FYLO's error codes; the other rows
   are tags at the places where Tachyon makes the failure. FYLO codes that are
   not listed are `failed`.

   | Code | FYLO codes | Tachyon failures |
   | --- | --- | --- |
   | `budget_spent` | `EDEADLINE` | the store deadline (`TY2160` "Managed storage operation deadline exceeded ...", also "during FYLO <operation>"); a SQLite statement stopped by its deadline; a call that starts after the shared deadline; the endpoint's own invocation deadline (HTTP 408) |
   | `busy` | `EROOTLOCKED` (after the 2-second wait of ADR 0128), `EBUSY`, `ETXNBUSY` | storage admission closed or full |
   | `conflict` | `EDOCEXISTS`, `EENGINE_CONCURRENT_WRITE`, `ENATIVE_CONCURRENT_WRITE` | a SQL unique or primary-key violation (the driver's `is_unique_violation`) |
   | `not_found` | `EINVALIDDOCID`, `ENATIVE_NOT_FOUND`, `ENOTFOUND`, `ECOLLECTION_NOT_FOUND`, `ENATIVE_UNSAFE_PATH` | none |
   | `too_large` | `ETXNLIMIT`, `EFRAME_REQUEST_TOO_LARGE`, `EFRAME_RESPONSE_TOO_LARGE`, `EQUERY_INPUT_SIZE`, `EQUERY_OUTPUT_SIZE`, `EQUERY_TERM_SIZE`, `EQUERYITEMTOOLARGE`, `EQUERYSNAPSHOTTOOLARGE`, `EINDEX_KEY_SIZE`, `EINDEX_SNAPSHOT_SIZE`, `ENATIVE_FILE_SIZE`, `EFORMAT_SIZE`, `EFORMAT_DEPTH`, `EFORMAT_NODES` | a command or result over 1 MiB; a result over 100 rows; a FYLO result over the byte limit or the scan bound; an answer too large for the endpoint (HTTP 502) |
   | `transaction_refused` | `ETXNSCOPE`, `ETXNLAYOUT`, `ETXNUNKNOWN`, `ETXNEXPIRED` | the open-transaction limit; an unknown, expired or settled transaction; a transaction on the FYLO datasource before FYLO transactions |
   | `unavailable` | `EPROTOCOL`, `EROOTCLOSED`, `EROOTLEASELOST` | none |
   | `failed` | every other code | everything else, including validation and configuration failures and a refused (403) endpoint call |

   A read that answers "missing" as `null` or `false` (find by id, delete of
   a missing id) still does not fail. FYLO's transaction errors arrive with
   the FYLO Store operations (TAC-FY2); whichever change lands second adds
   its rows to the same table.

3. **The wire.** A failed endpoint call keeps its HTTP status and answers
   `{"code":"<code>","error":"Yon managed resource operation failed."}`.
   The `handler.storage_failed` log line of ADR 0128 gains the same
   `"code"`. A route answer built from a resource error (for example Go's
   500) does not change.

4. **Each prelude.** A prelude reads `code` from a non-200 answer that is
   otherwise valid. A code that is not a string of the set is `failed`. The
   errors a prelude makes itself:
   - an invalid endpoint or token, an encoding failure or an invalid answer:
     `failed`;
   - a request body over 1 MiB: `too_large`;
   - no connection, or a failed write or read: `unavailable`;
   - the prelude's own timeout: `budget_spent`. In PHP and Dart a timeout
     while the connection opens is `unavailable`, because their socket
     clients do not tell it apart; nothing was sent, so nothing was written.

5. **The API is additive.** The message text and the old types stay, so old
   handler code compiles and old catches still catch.

   | Language | How a handler reads the code |
   | --- | --- |
   | Go | `YonResourceError{Code string}`; `err.(YonResourceError).Code` |
   | Rust | `YonResourceError::code(&self) -> &'static str` |
   | Java | `YonResourceException extends IllegalStateException`, `code()` |
   | Kotlin | `YonResourceException : IllegalStateException`, `code` |
   | C#, F# | `YonResourceException : InvalidOperationException`, `Code` |
   | PHP | `YonResourceException extends RuntimeException`, `resourceCode()` (PHP's `getCode()` is a final integer) |
   | Dart | `YonResourceError extends StateError`, `code` |
   | JavaScript, TypeScript | the same `Error`, with a `code` string |
   | Python | `YonResourceError(RuntimeError)`, `code` |
   | Ruby | `YonResourceError#code` |

   Lua has no managed storage, only gateway calls, so it has no resource
   error. Gateway, topic and log answers do not change.

6. **Rust keeps the unit value.** Rust's `YonResourceError` was a unit
   struct, so handler code writes `Err(YonResourceError)` and
   `map_err(|_| YonResourceError)`. It is now a struct with a private code,
   and the prelude also declares `pub const YonResourceError: YonResourceError`
   (the `failed` error) in the value namespace, so that code still compiles
   and still matches. This is a file-level constant, which
   [Engineering Standards](../ENGINEERING_STANDARDS.md#file-level-variables)
   allows for the code that creates the framework's runtime: the Yon prelude
   is that code, not application code.

## Consequences

- An application can branch on the code: start again after `busy`,
  `conflict`, `unavailable` and, with care, `budget_spent`; fix the request
  after `too_large`, `not_found` or `transaction_refused`.
- `budget_spent` does not prove that nothing was written, as before
  (ADR 0128).
- The set is closed. A new code needs a new ADR or an amendment, and every
  prelude must learn it; until then an older prelude reports it as `failed`.
- The codes say nothing secret: an operator still reads the details in the
  `handler.storage_failed` line.

## Evidence

- `resource_error_code::tests`: the stable texts and the FYLO table.
- `storage::tests::resource_codes`: SQLite conflict, too large (result and
  command), unknown and over-limit transactions, a deadline, a call after the
  shared deadline, a plain failure, and FYLO `not_found` (with `fylo`).
- `storage::fylo::resource_code_tests`: FYLO errors keep their message and
  gain their code; a deadline that stops FYLO is `budget_spent`.
- `handler::resources::access_failure::tests` and
  `handler::resources::tests::a_failed_store_call_answers_its_code`: the body
  and the status-only codes.
- `resource_error_codes` (integration, one test per language): a Go, Rust,
  Java, Kotlin, C#, PHP, Dart, JavaScript, Python and Ruby handler sees
  `conflict`, `too_large`, `transaction_refused` and `budget_spent` from real
  SQLite, `not_found` from real FYLO, and the unchanged message;
  `rust_handler_code_written_before_the_codes_still_compiles`.
- `tests/managed-resource-transports.mjs`: for Java, Kotlin, C#, PHP and Go,
  every code, an unknown code, a code that is not a string and a missing code
  against a fake endpoint, and a stalled answer is `budget_spent`.
