import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, test } from 'node:test';

class PinnedBuildFixture {
  static commit = 'abc1234';
  static folder = 'stand-in';

  /** @param {number} protocol */
  static standInFylo(protocol) {
    return `#!/bin/sh\necho '{"protocolVersion":${protocol},"op":"handshake","ok":true,"result":{}}'\n`;
  }

  /**
   * @param {{ protocol?: number, pinnedSum?: string, link?: boolean, missing?: boolean }} options
   * @returns {{ script: string, builds: string, install: string }}
   */
  static create(options) {
    const work = mkdtempSync(join(tmpdir(), 'install-fylo-test-'));
    const builds = join(work, 'builds');
    const folder = join(builds, PinnedBuildFixture.commit, PinnedBuildFixture.folder);
    mkdirSync(folder, { recursive: true });
    const text = PinnedBuildFixture.standInFylo(options.protocol ?? 1);
    const sum = createHash('sha256').update(text).digest('hex');
    PinnedBuildFixture.writeExecutable(folder, text, options);
    const script = join(work, 'install-fylo.mjs');
    writeFileSync(script, PinnedBuildFixture.pinnedScript(options.pinnedSum ?? sum));
    return { script, builds, install: join(work, 'installed') };
  }

  /** @param {string} folder @param {string} text @param {{ link?: boolean, missing?: boolean }} options */
  static writeExecutable(folder, text, options) {
    if (options.missing) return;
    const target = options.link ? join(folder, 'real-fylo') : join(folder, 'fylo');
    writeFileSync(target, text);
    chmodSync(target, 0o755);
    if (options.link) symlinkSync(target, join(folder, 'fylo'));
  }

  /** @param {string} sum */
  static pinnedScript(sum) {
    const here = dirname(fileURLToPath(import.meta.url));
    const original = readFileSync(join(here, 'install-fylo.mjs'), 'utf8');
    const platform = `${process.platform}-${process.arch}`;
    const build = `static build = {
    commit: '${PinnedBuildFixture.commit}',
    protocol: 1,
    targets: { '${platform}': ['${PinnedBuildFixture.folder}', 'fylo', '${sum}'] },
  };`;
    return original
      .replace(/static kind = '[a-z]+';/, "static kind = 'build';")
      .replace(/static build = \{[\s\S]*?\n {2}\};/, build);
  }

  /** @param {{ script: string, builds: string, install: string }} fixture @param {string} argument */
  static run(fixture, argument) {
    try {
      const output = execFileSync(process.execPath, [fixture.script, argument], {
        encoding: 'utf8',
        env: { ...process.env, TAC_FYLOBUILDS: fixture.builds },
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      return { ok: true, text: output };
    } catch (error) {
      return { ok: false, text: String(error.stderr) };
    }
  }
}

describe('install-fylo build pin', { skip: process.platform === 'win32' }, () => {
  test('describes the pinned build without touching anything', () => {
    const fixture = PinnedBuildFixture.create({});
    const answer = PinnedBuildFixture.run(fixture, '--describe');
    assert.equal(answer.text.trim(), `build abc1234 ${join(fixture.builds, 'abc1234')}`);
    assert.equal(existsSync(fixture.install), false);
  });

  test('copies a build whose checksum matches and whose handshake names the pinned protocol', () => {
    const fixture = PinnedBuildFixture.create({});
    const answer = PinnedBuildFixture.run(fixture, fixture.install);
    assert.equal(answer.ok, true, answer.text);
    assert.match(answer.text, /Verified FYLO abc1234 stand-in\/fylo: checksum and machine protocol 1/);
    assert.equal(existsSync(join(fixture.install, 'probe-root')), false);
  });

  test('refuses a build whose checksum differs and writes nothing', () => {
    const fixture = PinnedBuildFixture.create({ pinnedSum: '0'.repeat(64) });
    const answer = PinnedBuildFixture.run(fixture, fixture.install);
    assert.equal(answer.ok, false);
    assert.match(answer.text, /FYLO checksum mismatch/);
    assert.equal(existsSync(fixture.install), false);
  });

  test('refuses a missing build', () => {
    const fixture = PinnedBuildFixture.create({ missing: true });
    const answer = PinnedBuildFixture.run(fixture, fixture.install);
    assert.equal(answer.ok, false);
    assert.match(answer.text, /pinned FYLO develop build is missing/);
  });

  test('refuses a build that is a link', () => {
    const fixture = PinnedBuildFixture.create({ link: true });
    const answer = PinnedBuildFixture.run(fixture, fixture.install);
    assert.equal(answer.ok, false);
    assert.match(answer.text, /not a link/);
  });

  test('refuses a build whose handshake names another protocol', () => {
    const fixture = PinnedBuildFixture.create({ protocol: 2 });
    const answer = PinnedBuildFixture.run(fixture, fixture.install);
    assert.equal(answer.ok, false);
    assert.match(answer.text, /machine-protocol 1 capability probe/);
  });
});
