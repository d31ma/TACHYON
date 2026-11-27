# ADR 0148: A Server Can Leave Schedule Workers to an Outside Scheduler

- Status: Accepted
- Date: 2026-11-21
- Amends: [ADR 0026](0026-json-results-and-separated-configuration.md)
  (`yon.server` gets one more member, `workers`)
- Requested by: Mesail MSL-K27 (ticket TAC-WK1)

## Context

`ty start` runs every worker under `server/workers` on its schedule, from
`@Schedule({...})` or `.tachyonrc.workers`. On a long-running server this is
what an application wants.

On a host such as AWS Lambda, `ty start` runs inside each function instance.
Such a host already has its own scheduler (for example EventBridge), and the
application asks it to call the routes that do the scheduled work. Then the
work runs twice: once from the outside scheduler and once from the workers
inside the function. An instance may also be frozen between requests, so the
timers inside it do not run on time. Mesail found this in MSL-K27 and had to
add its own switch to every worker as a temporary bridge.

## Decision

`server/application.json` and its profiles accept `server.workers`, a
boolean:

```json
{
  "server": { "workers": false }
}
```

- `true`, or no value: `ty start` runs every scheduled worker, as before.
- `false`: `ty start` starts no scheduled worker. The worker files are still
  discovered and checked by the build, so a broken worker still fails the
  build. The server prints one line on standard error that says the workers
  are off and how many will not run.
- Any other value, such as the string `"false"`, stops `ty start` with
  `TY2010` and the message `yon.server.workers must be true or false`.

The setting belongs in a profile, for example
`server/application-lambda.json`, that only the host with the outside
scheduler selects through `YON_PROFILE`. The base file keeps the default,
so local development still runs the workers.

`server` stays a closed set (ADR 0026): it now holds `host`, `port`,
`hosts`, `origins`, `proxies`, `environment`, `maxbody` and `workers`.

## Consequences

- An application on Lambda turns its in-function workers off with one
  setting, with no code in each worker. Mesail can remove its bridge.
- The outside scheduler becomes the only thing that runs the scheduled work
  on that host. If it is not set up, the work does not run at all. The
  standard error line on start makes that visible in the host's logs.
- Topic deliveries, the file watcher and the producer reaper still start.
  The setting covers scheduled workers only.

## Out of scope

- Turning off single workers. The switch is for the whole server.
- A cron field, or running the workers from the outside scheduler through
  Tachyon itself.
