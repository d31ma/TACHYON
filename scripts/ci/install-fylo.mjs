import { createHash } from 'node:crypto';
import { chmod, mkdir, writeFile, appendFile, rm } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';

class FyloRelease {
  static version = 'v26.45.07';

  /** @type {Record<string, [string, string]>} */
  static targets = {
    'linux-arm64': ['fylo-linux-arm64', '45b83a0211a45bc3d50ee487867513c2c135b53bc914c728f78311f8b712667c'],
    'linux-x64': ['fylo-linux-x64', 'c198bd3cb9651578756229eddde732ab810422c6ba5fc4a3cb4678d542bdc1cd'],
    'darwin-arm64': ['fylo-macos-arm64', '3a9451ccfa8b95474c2cb1871d86d2aafada637d0bf31f50dbce01e1de83a868'],
    'darwin-x64': ['fylo-macos-x64', 'f5b0f08d5ae1ee927727497203124a7287c5e6652ead54ca36ee7de84464a62a'],
    'win32-x64': ['fylo-windows-x64.exe', '791b6095c7a191c2cd558fa4319a35a6d07c9591e76691885d6999cd3ff68d9e'],
    'win32-arm64': ['fylo-windows-x64.exe', '791b6095c7a191c2cd558fa4319a35a6d07c9591e76691885d6999cd3ff68d9e'],
  };
}

async function downloadAsset(asset) {
  const response = await fetch(`https://github.com/d31ma/FYLO/releases/download/${FyloRelease.version}/${asset}`, {
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

async function installExecutable(directory, bytes) {
  await mkdir(directory, { recursive: true });
  const executable = join(directory, process.platform === 'win32' ? 'fylo.exe' : 'fylo');
  await writeFile(executable, bytes, { flag: 'wx', mode: 0o755 });
  await chmod(executable, 0o755);
  return executable;
}

async function probeExecutable(executable, directory) {
  const probeRoot = join(directory, 'probe-root');
  await mkdir(probeRoot, { recursive: true });
  try {
    const answer = JSON.parse(execFileSync(executable, [
      'exec', '--request', '{"op":"createCollection","collection":"probe"}', '--root', probeRoot,
    ], { encoding: 'utf8', timeout: 30_000, maxBuffer: 256 * 1024 }));
    if (answer.protocolVersion !== 1 || answer.op !== 'createCollection' || answer.ok !== true) {
      throw new Error('Pinned FYLO failed its machine-protocol capability probe');
    }
  } finally {
    await rm(probeRoot, { recursive: true, force: true });
    for (const suffix of ['', '.json']) {
      await rm(join(directory, `.probe-root.fylo-root-owner.lock${suffix}`), { force: true });
    }
  }
}

async function main() {
  const target = FyloRelease.targets[`${process.platform}-${process.arch}`];
  if (!target) throw new Error('No qualified FYLO binary for this runner');
  const directoryArgument = process.argv[2];
  if (!directoryArgument) throw new Error('Pass an isolated installation directory');
  const directory = resolve(directoryArgument);
  const [asset, digest] = target;
  const bytes = await downloadAsset(asset);
  if (createHash('sha256').update(bytes).digest('hex') !== digest) {
    throw new Error('FYLO checksum mismatch; refusing to write or execute it');
  }
  const executable = await installExecutable(directory, bytes);
  await probeExecutable(executable, directory);
  if (process.env.GITHUB_PATH) await appendFile(process.env.GITHUB_PATH, `${directory}\n`);
  console.log(`Verified FYLO ${FyloRelease.version} ${asset}: checksum and machine protocol`);
}

await main();
