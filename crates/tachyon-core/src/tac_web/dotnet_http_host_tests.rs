#![allow(clippy::expect_used)]

use std::time::Duration;

struct DotnetHttpHostFixture;

impl DotnetHttpHostFixture {
    const CHECKS: &'static str = r"
/** @typedef {{url: string, options: RequestInit, accept: (value: unknown) => void}} HttpTicketRequest */
class DotnetHttpChecks {
  /** @type {HttpTicketRequest[]} */
  static requests = []
  /** @param {string} url @param {RequestInit} options @returns {Promise<Response>} */
  static fetch(url, options) {
    return new Promise((resolve, reject) => {
      DotnetHttpChecks.requests.push({ url, options, accept: value => resolve(new Response(JSON.stringify(value))) })
      options.signal.addEventListener('abort', () => reject(new DOMException('Cancelled', 'AbortError')), { once: true })
    })
  }
  /** @param {Partial<Record<string, unknown>>} values @returns {string} */
  static request(values = {}) {
    return JSON.stringify({ method: 'GET', path: '/api/probe', query: {}, headers: {}, redirect: 'error', credentials: 'omit', deadline: Date.now() + 10000, maxResponseBytes: 1048576, ...values })
  }
  /** @param {boolean} valid @param {string} message @returns {void} */
  static check(valid, message) { if (!valid) throw new Error(message) }
  /** @returns {Promise<void>} */
  static async run() {
    globalThis.location = new URL('https://test.example.invalid/')
    globalThis.fetch = (url, options) => DotnetHttpChecks.fetch(url, options)
    const host = new TacDotnetHttp()
    const first = host.start(DotnetHttpChecks.request({ publicRead: true }))
    const second = host.start(DotnetHttpChecks.request({ publicRead: true, revalidate: true }))
    DotnetHttpChecks.check(first !== second, 'HTTP ticket identities reused')
    const [a, b] = DotnetHttpChecks.requests
    DotnetHttpChecks.check(a.options.cache === 'default' && b.options.cache === 'no-cache', 'GET cache policy changed')
    DotnetHttpChecks.check(a.options.credentials === 'omit' && a.options.redirect === 'error' && a.options.mode === 'same-origin', 'HTTP security options changed')
    host.cancel(first)
    b.accept({ owner: 'second' })
    const cancelled = JSON.parse(await host.answer(first))
    DotnetHttpChecks.check(cancelled.error.code === 'CANCELLED', 'HTTP cancellation code lost')
    DotnetHttpChecks.check(JSON.parse(await host.answer(second)).value.owner === 'second', 'Cancellation crossed tickets')
    host.cancel(first)
    const ordinary = host.start(DotnetHttpChecks.request())
    const c = DotnetHttpChecks.requests.at(-1)
    DotnetHttpChecks.check(c.options.cache === 'no-store', 'Ordinary HTTP became cacheable')
    c.accept(null)
    DotnetHttpChecks.check(JSON.parse(await host.answer(ordinary)).value === null, 'Null HTTP value lost')
    const privateRead = host.start(DotnetHttpChecks.request({ publicRead: true, headers: { authorization: 'test authorization' } }))
    const d = DotnetHttpChecks.requests.at(-1)
    DotnetHttpChecks.check(d.options.cache === 'no-store', 'Authenticated read became public cache')
    d.accept({ private: true })
    await host.answer(privateRead)
    const refused = host.start(DotnetHttpChecks.request({ path: '//other.example.invalid/' }))
    DotnetHttpChecks.check(JSON.parse(await host.answer(refused)).error.code === 'HTTP_ORIGIN', 'Cross-origin path accepted')
    const expired = host.start(DotnetHttpChecks.request({ deadline: Date.now() - 1 }))
    DotnetHttpChecks.check(JSON.parse(await host.answer(expired)).error.code === 'DEADLINE', 'Deadline code lost')
    const full = []
    for (let index = 0; index < TacDotnetHttp.limit; index++) full.push(host.start(DotnetHttpChecks.request()))
    const capacity = host.start(DotnetHttpChecks.request())
    DotnetHttpChecks.check(JSON.parse(capacity).error.code === 'HTTP_CAPACITY', 'Ticket limit did not refuse before allocation')
    for (const ticket of full) host.cancel(ticket)
    const errors = await Promise.all(full.map(ticket => host.answer(ticket)))
    DotnetHttpChecks.check(errors.every(text => JSON.parse(text).error.code === 'CANCELLED'), 'Pending ticket did not cancel')
    const fresh = host.start(DotnetHttpChecks.request())
    DotnetHttpChecks.requests.at(-1).accept({ fresh: true })
    DotnetHttpChecks.check(JSON.parse(await host.answer(fresh)).value.fresh, 'Retired tickets retained capacity')
    DotnetHttpChecks.check(JSON.parse(await host.answer(first)).error.code === 'HTTP_MISSING', 'Retired ticket resurrected')
    console.log('PASS bounded .NET HTTP ticket host unit')
  }
}
await DotnetHttpChecks.run()
";

    fn run() -> String {
        let directory = tempfile::tempdir().expect("isolated HTTP host unit");
        let script = directory.path().join("DotnetHttpChecks.mjs");
        let source = format!(
            "{}\n{}\n{}",
            include_str!("../template/tac-http.js"),
            include_str!("../native/browser/tac-dotnet-http.js"),
            Self::CHECKS
        );
        std::fs::write(&script, source).expect("actual default HTTP and ticket host sources");
        let mut command = std::process::Command::new("node");
        command.arg(script);
        let output =
            crate::external_command::run_sync(&mut command, Duration::from_secs(20), 65_536)
                .expect("supervised controlled HTTP boundary unit");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn dotnet_http_host_keeps_ticket_ownership_bounds_and_default_fetch_policy() {
    assert!(DotnetHttpHostFixture::run().contains("PASS bounded .NET HTTP ticket host unit"));
}
