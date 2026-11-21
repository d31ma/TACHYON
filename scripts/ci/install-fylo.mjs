import { createHash } from 'node:crypto';
import { chmod, mkdir, writeFile, appendFile, rm, lstat, readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';

class FyloPin {
  /** @type {'release' | 'build'} */
  static kind = 'release';

  static release = {
    version: 'v26.45.07',
    protocol: 1,
    /** @type {Record<string, [string, string]>} */
    targets: {
      'linux-arm64': ['fylo-linux-arm64', '45b83a0211a45bc3d50ee487867513c2c135b53bc914c728f78311f8b712667c'],
      'linux-x64': ['fylo-linux-x64', 'c198bd3cb9651578756229eddde732ab810422c6ba5fc4a3cb4678d542bdc1cd'],
      'darwin-arm64': ['fylo-macos-arm64', '3a9451ccfa8b95474c2cb1871d86d2aafada637d0bf31f50dbce01e1de83a868'],
      'darwin-x64': ['fylo-macos-x64', 'f5b0f08d5ae1ee927727497203124a7287c5e6652ead54ca36ee7de84464a62a'],
      'win32-x64': ['fylo-windows-x64.exe', '791b6095c7a191c2cd558fa4319a35a6d07c9591e76691885d6999cd3ff68d9e'],
      'win32-arm64': ['fylo-windows-x64.exe', '791b6095c7a191c2cd558fa4319a35a6d07c9591e76691885d6999cd3ff68d9e'],
    },
  };

  static build = {
    commit: '',
    protocol: 1,
    /** @type {Record<string, [string, string, string]>} */
    targets: {
      'darwin-arm64': ['macos-arm64', 'fylo', ''],
      'linux-arm64': ['linux-arm64', 'fylo', ''],
      'win32-arm64': ['windows-arm64', 'fylo.exe', ''],
    },
  };

  static buildsFolder() {
    return process.env.TAC_FYLO_BUILDS || join(homedir(), 'Developer', 'AnvilTargets', 'fylo-builds');
  }

  static description() {
    if (FyloPin.kind === 'build') return `build ${FyloPin.build.commit} ${join(FyloPin.buildsFolder(), FyloPin.build.commit)}`;
    return `release ${FyloPin.release.version}`;
  }
}

async function downloadAsset(asset) {
  const response = await fetch(`https://github.com/d31ma/FYLO/releases/download/${FyloPin.release.version}/${asset}`, {
    signal: AbortSignal.timeout(60_000),
  });
  if (!response.ok || !response.body) throw new Error(`FYLO download failed (${response.status})`);
  const chunks = [];
  let length = 0;
  for await (const chunk of response.body) {
    length += chunk.length;
    if (length > 32 * 1024 * 1024) throw new Error('FYLO download exceeds its size budget');
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

async function readBuild(platform) {
  const target = FyloPin.build.targets[platform];
  if (!target) throw new Error(`The pinned FYLO develop build has no file for ${platform}`);
  if (!/^[0-9a-f]{7,40}$/.test(FyloPin.build.commit)) throw new Error('The FYLO develop build pin names no FYLO commit');
  const [folder, file, digest] = target;
  if (!/^[0-9a-f]{64}$/.test(digest)) throw new Error(`The FYLO develop build pin has no SHA-256 for ${platform}`);
  const path = join(FyloPin.buildsFolder(), FyloPin.build.commit, folder, file);
  const info = await lstat(path).catch(() => null);
  if (!info?.isFile()) throw new Error(`The pinned FYLO develop build is missing: ${path} (it must be a real file, not a link)`);
  if (info.size > 64 * 1024 * 1024) throw new Error('The FYLO develop build exceeds its size budget');
  return { bytes: await readFile(path), digest, name: `${FyloPin.build.commit} ${folder}/${file}` };
}

async function readRelease(platform) {
  const target = FyloPin.release.targets[platform];
  if (!target) throw new Error('No qualified FYLO binary for this runner');
  const [asset, digest] = target;
  return { bytes: await downloadAsset(asset), digest, name: `${FyloPin.release.version} ${asset}` };
}

async function installExecutable(directory, bytes) {
  await mkdir(directory, { recursive: true });
  const executable = join(directory, process.platform === 'win32' ? 'fylo.exe' : 'fylo');
  await writeFile(executable, bytes, { flag: 'wx', mode: 0o755 });
  await chmod(executable, 0o755);
  return executable;
}

async function probeExecutable(executable, directory, protocol) {
  const probeRoot = join(directory, 'probe-root');
  await mkdir(probeRoot, { recursive: true });
  try {
    const answer = JSON.parse(execFileSync(executable, [
      'exec', '--request', '{"op":"createCollection","collection":"probe"}', '--root', probeRoot,
    ], { encoding: 'utf8', timeout: 30_000, maxBuffer: 256 * 1024 }));
    if (answer.protocolVersion !== protocol || answer.op !== 'createCollection' || answer.ok !== true) {
      throw new Error(`Pinned FYLO failed its machine-protocol ${protocol} capability probe`);
    }
  } finally {
    await rm(probeRoot, { recursive: true, force: true });
    for (const suffix of ['', '.json']) {
      await rm(join(directory, `.probe-root.fylo-root-owner.lock${suffix}`), { force: true });
    }
  }
}

async function main() {
  if (process.argv[2] === '--describe') {
    console.log(FyloPin.description());
    return;
  }
  const directoryArgument = process.argv[2];
  if (!directoryArgument) throw new Error('Pass an isolated installation directory, or --describe');
  const directory = resolve(directoryArgument);
  const platform = `${process.platform}-${process.arch}`;
  const pinned = FyloPin.kind === 'build' ? await readBuild(platform) : await readRelease(platform);
  const got = createHash('sha256').update(pinned.bytes).digest('hex');
  if (got !== pinned.digest) {
    throw new Error(`FYLO checksum mismatch for ${pinned.name}: got ${got}, pinned ${pinned.digest}; refusing to write or execute it`);
  }
  const protocol = FyloPin.kind === 'build' ? FyloPin.build.protocol : FyloPin.release.protocol;
  const executable = await installExecutable(directory, pinned.bytes);
  await probeExecutable(executable, directory, protocol);
  if (process.env.GITHUB_PATH) await appendFile(process.env.GITHUB_PATH, `${directory}\n`);
  console.log(`Verified FYLO ${pinned.name}: checksum and machine protocol ${protocol}`);
}

await main();
