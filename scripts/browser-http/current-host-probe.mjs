import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';
import assert from 'node:assert/strict';

class BrowserHttpProbe {
  static source = new URL('../../crates/tachyon-core/src/template/tac-web.js', import.meta.url);

  static async run() {
    const source = await readFile(BrowserHttpProbe.source, 'utf8');
    const environment = { console, Symbol, Object, JSON };
    runInNewContext(`${source}\nglobalThis.probe = TacWebApis;`, environment);
    const request = { api: 'http', call: 'exchange', args: [{ method: 'GET', path: '/api/greeting', query: {}, headers: {}, credentials: 'omit', publicRead: false, revalidate: false }] };
    const answer = JSON.parse(environment.probe.answer(JSON.stringify(request)));
    console.log(JSON.stringify({ request: 'GET /api/greeting', answer }));
    assert.equal(answer.error, undefined, 'Production host must expose a default HTTP exchange');
    assert.equal(answer.value.message, 'Hello from Yon');
  }
}

await BrowserHttpProbe.run();
