import { createHash } from 'node:crypto';
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';

class LanguageToolchains {
  static platform = `${process.platform}-${process.arch}`;

  /** @type {Record<string, { url: string, sha256: string, bin: string }>} */
  static archives = {
    groovy: {
      url: 'https://archive.apache.org/dist/groovy/5.1.3/distribution/apache-groovy-binary-5.1.3.zip',
      sha256: '9f53e2c2a4e570f7ca8d04df64738c1d83af29834da7a6bcad9014601f67a014',
      bin: 'groovy-5.1.3/bin',
    },
    kotlin: {
      url: 'https://github.com/JetBrains/kotlin/releases/download/v2.1.10/kotlin-compiler-2.1.10.zip',
      sha256: 'c6e9e2636889828e19c8811d5ab890862538c89dc2a3101956dfee3c2a8ba6b1',
      bin: 'kotlinc/bin',
    },
  };

  /** @type {Record<string, Record<string, { url: string, sha256: string, bin: string }>>} */
  static platformArchives = {
    dart: {
      'linux-x64': {
        url: 'https://storage.googleapis.com/dart-archive/channels/stable/release/3.13.1/sdk/dartsdk-linux-x64-release.zip',
        sha256: '9251c4606ebb31480c45f430be69fce2697e3b888a0a828b9e16e7d23403d72a',
        bin: 'dart-sdk/bin',
      },
      'darwin-arm64': {
        url: 'https://storage.googleapis.com/dart-archive/channels/stable/release/3.13.1/sdk/dartsdk-macos-arm64-release.zip',
        sha256: '367984941d8d14c653789f6f787312e715e5c546f6e9f5d30d86615c692907a9',
        bin: 'dart-sdk/bin',
      },
      'win32-x64': {
        url: 'https://storage.googleapis.com/dart-archive/channels/stable/release/3.13.1/sdk/dartsdk-windows-x64-release.zip',
        sha256: 'f3fd0fa8f3bc7982bbf08eb1bca84148fb4ee0767e75faa07ec9c11c40ab2ff7',
        bin: 'dart-sdk/bin',
      },
    },
    nextest: {
      'linux-x64': {
        url: 'https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-x86_64-unknown-linux-gnu.tar.gz',
        sha256: '682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428',
        bin: '.',
      },
      'darwin-arm64': {
        url: 'https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-universal-apple-darwin.tar.gz',
        sha256: '39785160b3c2f6ed9a765049cf4fa79f3b39aa02eb7598a5a0e2a1a0b9ffb9a8',
        bin: '.',
      },
      'win32-x64': {
        url: 'https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-x86_64-pc-windows-msvc.zip',
        sha256: '0fa689815c8157e4633225b6b173184b3d546eb6ffb754c3d0e6ea5973284a20',
        bin: '.',
      },
    },
  };

  static pyodide = {
    url: 'https://github.com/pyodide/pyodide/releases/download/314.0.7/pyodide-core-314.0.7.tar.bz2',
    sha256: '2abdcc2e35208af406e07724cffa85bc582ced97e9028383ecf5462541393f95',
    version: '314.0.7',
    files: ['pyodide.mjs', 'pyodide.asm.mjs', 'pyodide.asm.wasm', 'python_stdlib.zip', 'pyodide-lock.json'],
  };

  static luaSources = 'runtimes/lua-web/sources.txt';

  static usage = 'Usage: node scripts/ci/install-toolchains.mjs <directory> <groovy|kotlin|dart|lua|pyodide|nextest>...';

  /** @param {string} url @param {string} file @param {string} sha256 */
  static download(url, file, sha256) {
    execFileSync('curl', ['--fail', '--location', '--silent', '--show-error', '--retry', '3', '--max-time', '900', '--output', file, url], { stdio: 'inherit' });
    const actual = createHash('sha256').update(readFileSync(file)).digest('hex');
    if (actual !== sha256) throw new Error(`Checksum mismatch for ${url}: expected ${sha256}, got ${actual}`);
  }

  /** @param {string} file @param {string} into */
  static unpack(file, into) {
    mkdirSync(into, { recursive: true });
    const zip = file.endsWith('.zip');
    const program = process.platform === 'win32' ? join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'tar.exe') : zip ? 'unzip' : 'tar';
    const args = program === 'unzip' ? ['-q', file, '-d', into] : ['-xf', file, '-C', into];
    execFileSync(program, args, { stdio: 'inherit' });
  }

  /** @param {string} bin */
  static addToPath(bin) {
    if (process.env.GITHUB_PATH) appendFileSync(process.env.GITHUB_PATH, `${bin}\n`);
    console.log(`PATH += ${bin}`);
  }

  /** @param {string} directory @param {string} name @param {{ url: string, sha256: string, bin: string }} archive */
  static installArchive(directory, name, archive) {
    const into = join(directory, name);
    const file = join(directory, archive.url.split('/').pop() ?? name);
    LanguageToolchains.download(archive.url, file, archive.sha256);
    LanguageToolchains.unpack(file, into);
    LanguageToolchains.addToPath(resolve(into, archive.bin));
  }

  /** @param {string} directory @param {string} name */
  static installPlatformArchive(directory, name) {
    const archive = LanguageToolchains.platformArchives[name][LanguageToolchains.platform];
    if (!archive) throw new Error(`No pinned ${name} build for ${LanguageToolchains.platform}`);
    LanguageToolchains.installArchive(directory, name, archive);
  }

  static cacheRoot() {
    const home = process.env.HOME || process.env.USERPROFILE || homedir();
    const roots = {
      darwin: () => join(home, 'Library', 'Caches', 'Tachyon'),
      win32: () => join(process.env.LOCALAPPDATA || join(home, 'AppData', 'Local'), 'Tachyon', 'Cache'),
      linux: () => join(process.env.XDG_CACHE_HOME || join(home, '.cache'), 'tachyon'),
    };
    return process.env.TAC_CACHE_DIR || (roots[process.platform] ?? roots.linux)();
  }

  /** @param {string} directory */
  static installPyodide(directory) {
    const { url, sha256, version, files } = LanguageToolchains.pyodide;
    const file = join(directory, url.split('/').pop() ?? 'pyodide.tar.bz2');
    const unpacked = join(directory, 'pyodide');
    LanguageToolchains.download(url, file, sha256);
    LanguageToolchains.unpack(file, unpacked);
    const target = join(LanguageToolchains.cacheRoot(), 'python-web', version);
    mkdirSync(target, { recursive: true });
    for (const name of files) copyFileSync(join(unpacked, 'pyodide', name), join(target, name));
    console.log(`Pyodide ${version} core files -> ${target}`);
  }

  /** @param {string} source */
  static windowsCompiler(source) {
    const objects = readdirSync(source).filter(name => name.endsWith('.c') && name !== 'luac.c').map(name => join(source, name));
    for (const compiler of ['gcc', 'clang']) {
      try {
        execFileSync(compiler, ['-O2', '-o', join(source, 'lua.exe'), ...objects], { stdio: 'inherit' });
        return compiler;
      } catch (error) {
        console.log(`${compiler} could not build Lua: ${error.message}`);
      }
    }
    throw new Error('No C compiler built Lua on this Windows runner');
  }

  /** @param {string} directory */
  static installLua(directory) {
    const line = readFileSync(LanguageToolchains.luaSources, 'utf8').split('\n').find(entry => entry.startsWith('lua '));
    if (!line) throw new Error(`${LanguageToolchains.luaSources} has no lua line`);
    const [, version, sha256, url] = line.trim().split(/\s+/);
    const file = join(directory, `lua-${version}.tar.gz`);
    LanguageToolchains.download(url, file, sha256);
    LanguageToolchains.unpack(file, directory);
    const tree = join(directory, `lua-${version}`);
    const bin = process.platform === 'win32' ? join(tree, 'src') : join(directory, 'lua', 'bin');
    const builders = {
      win32: () => LanguageToolchains.windowsCompiler(join(tree, 'src')),
      darwin: () => LanguageToolchains.make(tree, 'macosx', directory),
      linux: () => LanguageToolchains.make(tree, 'linux', directory),
    };
    (builders[process.platform] ?? builders.linux)();
    execFileSync(join(bin, process.platform === 'win32' ? 'lua.exe' : 'lua'), ['-v'], { stdio: 'inherit' });
    LanguageToolchains.addToPath(bin);
  }

  /** @param {string} tree @param {string} target @param {string} directory */
  static make(tree, target, directory) {
    execFileSync('make', ['-C', tree, target], { stdio: 'inherit' });
    execFileSync('make', ['-C', tree, 'install', `INSTALL_TOP=${join(directory, 'lua')}`], { stdio: 'inherit' });
  }

  static main() {
    const [directoryArgument, ...tools] = process.argv.slice(2);
    if (!directoryArgument || tools.length === 0) throw new Error(LanguageToolchains.usage);
    const directory = resolve(directoryArgument);
    mkdirSync(directory, { recursive: true });
    /** @type {Record<string, () => void>} */
    const installers = {
      groovy: () => LanguageToolchains.installArchive(directory, 'groovy', LanguageToolchains.archives.groovy),
      kotlin: () => LanguageToolchains.installArchive(directory, 'kotlin', LanguageToolchains.archives.kotlin),
      dart: () => LanguageToolchains.installPlatformArchive(directory, 'dart'),
      nextest: () => LanguageToolchains.installPlatformArchive(directory, 'nextest'),
      lua: () => LanguageToolchains.installLua(directory),
      pyodide: () => LanguageToolchains.installPyodide(directory),
    };
    for (const tool of tools) {
      const install = installers[tool];
      if (!install) throw new Error(`Unknown toolchain '${tool}'. ${LanguageToolchains.usage}`);
      if (existsSync(join(directory, tool))) throw new Error(`${join(directory, tool)} already exists`);
      console.log(`::group::Install ${tool}`);
      install();
      console.log('::endgroup::');
    }
  }
}

LanguageToolchains.main();
