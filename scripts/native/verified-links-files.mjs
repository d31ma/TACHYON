#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

class VerifiedLinksFiles {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static defaultHost = 'd31ma.github.io';
  static linkGateId = 'dev.tachyon.link-gate';
  static windowsGateId = 'dev.tachyon.browser-gate-package';
  static windowsGatePublisher = 'CN=Tachyon Browser Gate';
  static names = ['apple-app-site-association', 'assetlinks.json', 'windows-app-web-link'];
  static usage = `Usage: node scripts/native/verified-links-files.mjs [--host <host>] [--out <folder>] [--compare]

Builds the link fixtures for <host> (default ${VerifiedLinksFiles.defaultHost}) and writes the three
verification files the host must serve under /.well-known/, plus .nojekyll, into <folder>.
  TAC_APPLE_TEAM_ID        the team in the apple-app-site-association app ID
  TAC_ANDROID_CERT_SHA256  the fingerprints of the APK signing certificates
With --compare, it fetches https://<host>/.well-known/<file> and fails when a served file differs.`;

  static options(argv) {
    const options = { host: VerifiedLinksFiles.defaultHost, out: null, compare: false };
    for (let index = 0; index < argv.length; index++) {
      const argument = argv[index];
      if (argument === '--host') options.host = argv[++index];
      else if (argument === '--out') options.out = path.resolve(argv[++index]);
      else if (argument === '--compare') options.compare = true;
      else {
        console.error(VerifiedLinksFiles.usage);
        process.exit(argument === '--help' ? 0 : 2);
      }
    }
    assert.match(options.host ?? '', /^[a-z0-9-]+(\.[a-z0-9-]+)+$/, '--host takes a lowercase host name with a dot');
    return options;
  }

  static run(program, args, environment = {}) {
    const env = { ...process.env, ...environment };
    for (const [name, value] of Object.entries(environment)) if (value === null) delete env[name];
    const result = spawnSync(program, args, {
      cwd: VerifiedLinksFiles.repo, encoding: 'utf8', timeout: 600000, maxBuffer: 8 * 1024 * 1024, env,
    });
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout + result.stderr;
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const { run } = VerifiedLinksFiles;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static write(project, relative, source) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static stage(project, id, host) {
    const { write } = VerifiedLinksFiles;
    const application = { id, version: '0.0.1', build: 1, links: { schemes: ['linkgate'], hosts: [host] } };
    write(project, 'client/manifest.json', JSON.stringify({ name: 'LinkGate', start_url: '/', tac: { origin: `https://${host}`, application } }));
    write(project, 'client/pages/tac.html', '<link-home />');
    write(project, 'client/components/link/home/tac.html', '<main><h1>Link home</h1></main>');
  }

  static wellKnown(project) {
    return path.join(project, 'dist/.well-known');
  }

  static build(project, environment) {
    const output = VerifiedLinksFiles.run(VerifiedLinksFiles.binary(), ['build', project], environment);
    return { output, folder: VerifiedLinksFiles.wellKnown(project) };
  }

  static generate(host, out) {
    const { stage, build, linkGateId, windowsGateId, windowsGatePublisher, names } = VerifiedLinksFiles;
    assert.ok(process.env.TAC_APPLE_TEAM_ID, 'TAC_APPLE_TEAM_ID is required');
    assert.ok(process.env.TAC_ANDROID_CERT_SHA256, 'TAC_ANDROID_CERT_SHA256 is required');
    const scratch = mkdtempSync(path.join(tmpdir(), 'tachyon-verified-links-'));
    try {
      const mobile = path.join(scratch, 'mobile');
      stage(mobile, linkGateId, host);
      const apple = build(mobile, { TAC_WINDOWS_CERTIFICATE_PUBLISHER: null });
      const windows = path.join(scratch, 'windows');
      stage(windows, windowsGateId, host);
      const packaged = build(windows, { TAC_APPLE_TEAM_ID: null, TAC_ANDROID_CERT_SHA256: null, TAC_WINDOWS_CERTIFICATE_PUBLISHER: windowsGatePublisher });
      const target = path.join(out, '.well-known');
      mkdirSync(target, { recursive: true });
      for (const [name, folder] of [[names[0], apple.folder], [names[1], apple.folder], [names[2], packaged.folder]]) {
        assert.ok(existsSync(path.join(folder, name)), `the web build wrote no .well-known/${name}:\n${apple.output}${packaged.output}`);
        copyFileSync(path.join(folder, name), path.join(target, name));
      }
      writeFileSync(path.join(out, '.nojekyll'), '');
    } finally {
      rmSync(scratch, { recursive: true, force: true });
    }
    VerifiedLinksFiles.check(out);
  }

  static check(out) {
    const { linkGateId, windowsGateId, names } = VerifiedLinksFiles;
    const read = name => JSON.parse(readFileSync(path.join(out, '.well-known', name), 'utf8'));
    const apple = read(names[0]);
    assert.deepEqual(apple.applinks.details[0].appIDs, [`${process.env.TAC_APPLE_TEAM_ID}.${linkGateId}`]);
    const android = read(names[1]);
    assert.equal(android[0].target.package_name, linkGateId.replaceAll('-', '_'));
    assert.ok(android[0].target.sha256_cert_fingerprints.length > 0);
    const windows = read(names[2]);
    assert.match(windows[0].packageFamilyName, new RegExp(`^${windowsGateId.replaceAll('.', '\\.')}_[0-9a-z]{13}$`));
    for (const name of names) console.log(`${name}: ${JSON.stringify(read(name))}`);
  }

  static async compare(host, out) {
    const differences = [];
    for (const name of VerifiedLinksFiles.names) {
      const address = `https://${host}/.well-known/${name}`;
      const response = await fetch(address, { redirect: 'manual' });
      const served = response.status === 200 ? JSON.parse(await response.text()) : null;
      const expected = JSON.parse(readFileSync(path.join(out, '.well-known', name), 'utf8'));
      const same = JSON.stringify(served) === JSON.stringify(expected);
      console.log(`${same ? 'OK' : 'DIFFERS'}: ${address} answered ${response.status} ${response.headers.get('content-type') ?? ''}`);
      if (!same) differences.push(name);
    }
    assert.deepEqual(differences, [], `https://${host} does not serve the files in ${out}`);
  }

  static async main() {
    const options = VerifiedLinksFiles.options(process.argv.slice(2));
    const out = options.out ?? mkdtempSync(path.join(tmpdir(), 'tachyon-verified-links-site-'));
    VerifiedLinksFiles.generate(options.host, out);
    console.log(`Wrote the files for https://${options.host} to ${out}`);
    if (options.compare) await VerifiedLinksFiles.compare(options.host, out);
  }
}

await VerifiedLinksFiles.main();
