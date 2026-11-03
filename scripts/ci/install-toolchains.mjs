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
      'linux-arm64': {
        url: 'https://storage.googleapis.com/dart-archive/channels/stable/release/3.13.1/sdk/dartsdk-linux-arm64-release.zip',
        sha256: '5141d5ac62dabfcf0d3dd8fbf5c4d143d0092ef42fad0f25f6cf6587e5cfc3bf',
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
      'win32-arm64': {
        url: 'https://storage.googleapis.com/dart-archive/channels/stable/release/3.13.1/sdk/dartsdk-windows-arm64-release.zip',
        sha256: '7fafce047ce6c1f31eac3c256099c603ec31ed229aa06860cb232a3de8cf3897',
        bin: 'dart-sdk/bin',
      },
    },
    nextest: {
      'linux-x64': {
        url: 'https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-x86_64-unknown-linux-gnu.tar.gz',
        sha256: '682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428',
        bin: '.',
      },
      'linux-arm64': {
        url: 'https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-aarch64-unknown-linux-gnu.tar.gz',
        sha256: 'b2e33d7c72de7ade0ff7b3a948ac37516b24f8a836b7a8870c1f634a94be9de9',
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
      'win32-arm64': {
        url: 'https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-aarch64-pc-windows-msvc.zip',
        sha256: '8a071bd8190b287863c8aaa0b20fc56485f9190f0fc02206f48a0b0af4416de4',
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

  static teaVm = {
    url: 'https://repo1.maven.org/maven2/org/teavm',
    version: '0.15.0',
    jars: [
      ['cli', 'cli-0.15.0-all.jar', 'c6a1ecb81dd90b66985ac7b0f09db55bec2076fa024428c7fe72b800b6b48cae'],
      ['teavm-classlib', 'teavm-classlib-0.15.0.jar', '7d805be4670a892a34c89d9d750a8afd65e2f8201b7b3a10343634ea19d1c410'],
      ['teavm-jso-impl', 'teavm-jso-impl-0.15.0.jar', '9a8d15205807b70d7efdfbd4c1356304954e20146e464aec3830dbc6616ef57c'],
      ['teavm-metaprogramming-impl', 'teavm-metaprogramming-impl-0.15.0.jar', '616d30aa28f7b0180a61c2289b11c77bf84c715bd79f255e1e42400edb5b6d70'],
      ['teavm-platform', 'teavm-platform-0.15.0.jar', '999d369ab15aa83c17987d048ba59560759169891de7e13a5a1c2bd93dd060a8'],
    ],
  };

  static kotlinWasmStdlib = {
    url: 'https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-stdlib-wasm-js/2.1.10/kotlin-stdlib-wasm-js-2.1.10.klib',
    sha256: 'db9ab6772a2d3af9121d570a058cc64cf36893e1c089726b799b4d1252184ca2',
  };

  static luaSources = 'runtimes/lua-web/sources.txt';

  static usage = 'Usage: node scripts/ci/install-toolchains.mjs <directory> <groovy|kotlin|kotlin-wasm|dart|lua|pyodide|nextest|teavm>...';

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

  /** @param {string} name @param {string} value */
  static setEnvironment(name, value) {
    if (process.env.GITHUB_ENV) appendFileSync(process.env.GITHUB_ENV, `${name}=${value}\n`);
    console.log(`${name}=${value}`);
  }

  /** @param {string} directory */
  static installTeaVm(directory) {
    const { url, version, jars } = LanguageToolchains.teaVm;
    const into = join(directory, 'teavm');
    mkdirSync(into, { recursive: true });
    for (const [artifact, jar, sha256] of jars) LanguageToolchains.download(`${url}/${artifact}/${version}/${jar}`, join(into, jar), sha256);
    LanguageToolchains.setEnvironment('TAC_TEAVM_HOME', into);
  }

  /** @param {string} directory */
  static installKotlinWasmStdlib(directory) {
    const { url, sha256 } = LanguageToolchains.kotlinWasmStdlib;
    const into = join(directory, 'kotlin-wasm');
    mkdirSync(into, { recursive: true });
    const file = join(into, url.split('/').pop() ?? 'kotlin-stdlib-wasm-js.klib');
    LanguageToolchains.download(url, file, sha256);
    LanguageToolchains.setEnvironment('TAC_KOTLIN_WASM_STDLIB', file);
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
      teavm: () => LanguageToolchains.installTeaVm(directory),
      'kotlin-wasm': () => LanguageToolchains.installKotlinWasmStdlib(directory),
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
