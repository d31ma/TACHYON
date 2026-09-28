import { createHash } from 'node:crypto';
import { appendFile, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';

class EmscriptenInstall {
  static sourcesFile = new URL('../../runtimes/ruby-web/sources.txt', import.meta.url);

  /** @param {string} name */
  static async pinned(name) {
    const text = await readFile(this.sourcesFile, 'utf8');
    const line = text.split('\n').find((entry) => entry.split(/\s+/)[0] === name);
    if (!line) throw new Error(`runtimes/ruby-web/sources.txt does not pin ${name}`);
    const [, version, checksum, url] = line.trim().split(/\s+/);
    return { version, checksum, url };
  }

  /** @param {{ checksum: string, url: string }} emsdk */
  static async download(emsdk) {
    const response = await fetch(emsdk.url, { signal: AbortSignal.timeout(120_000) });
    if (!response.ok) throw new Error(`emsdk download failed (${response.status})`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (createHash('sha256').update(bytes).digest('hex') !== emsdk.checksum) {
      throw new Error('emsdk checksum mismatch; refusing to unpack or run it');
    }
    return bytes;
  }

  /**
   * @param {string} directory
   * @param {Buffer} bytes
   */
  static async unpack(directory, bytes) {
    await mkdir(join(directory, 'emsdk'), { recursive: true });
    await writeFile(join(directory, 'emsdk.tar.gz'), bytes, { flag: 'wx' });
    execFileSync('tar', ['-xzf', 'emsdk.tar.gz', '-C', 'emsdk', '--strip-components=1'], {
      cwd: directory, stdio: 'inherit',
    });
    await rm(join(directory, 'emsdk.tar.gz'));
    return join(directory, 'emsdk');
  }

  /**
   * @param {string} emsdk
   * @param {string} version
   */
  static activate(emsdk, version) {
    const python = process.platform === 'win32' ? 'python' : 'python3';
    for (const step of ['install', 'activate']) {
      execFileSync(python, ['emsdk.py', step, version], { cwd: emsdk, stdio: 'inherit' });
    }
  }

  /** @param {string} emsdk */
  static async publish(emsdk) {
    const compiler = join(emsdk, 'upstream', 'emscripten');
    const config = join(emsdk, '.emscripten');
    if (process.env.GITHUB_PATH) await appendFile(process.env.GITHUB_PATH, `${compiler}\n`);
    if (process.env.GITHUB_ENV) await appendFile(process.env.GITHUB_ENV, `EMSDK=${emsdk}\nEM_CONFIG=${config}\n`);
    const emcc = process.platform === 'win32' ? 'emcc.bat' : 'emcc';
    const printed = execFileSync(join(compiler, emcc), ['--version'], {
      encoding: 'utf8', env: { ...process.env, EM_CONFIG: config }, shell: process.platform === 'win32',
    });
    console.log(printed.split('\n')[0]);
  }

  static async main() {
    const directoryArgument = process.argv[2];
    if (!directoryArgument) throw new Error('Pass an empty installation directory');
    const directory = resolve(directoryArgument);
    const emsdk = await this.pinned('emsdk');
    const emscripten = await this.pinned('emscripten');
    const folder = await this.unpack(directory, await this.download(emsdk));
    this.activate(folder, emscripten.version);
    await this.publish(folder);
  }
}

await EmscriptenInstall.main();
