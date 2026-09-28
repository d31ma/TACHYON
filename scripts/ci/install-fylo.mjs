import { createHash } from 'node:crypto';
import { chmod, mkdir, writeFile, appendFile, rm } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';

class FyloRelease {
  /** @type {Record<string, [string, string]>} */
  static targets = {
    'linux-arm64': ['fylo-linux-arm64', 'ec161767b411b6a81d59771f43a094d4da22796c3c71301f2cc2a8f0a4db9d17'],
    'linux-x64': ['fylo-linux-x64', '8d60479cc189afb991d969917158d93d78897dcd0ca3aa7e062be186982644e8'],
    'darwin-arm64': ['fylo-macos-arm64', '2c78f91130863ad797cf7a6062aa1245fa638857fee63b8671fa9e2748974fa3'],
    'darwin-x64': ['fylo-macos-x64', '551e72291a7085536f5de360807dbe93a149826f2c0e6bbd7fe56b2e34abc545'],
    'win32-x64': ['fylo-windows-x64.exe', 'd57a612d5abb770bc3e228a08b6a628341364b19fe8c62a6418bbb178ef41f34'],
  };
}

async function downloadAsset(asset) {
  const response = await fetch(`https://github.com/d31ma/FYLO/releases/download/v26.33.02/${asset}`, {
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
  console.log(`Verified FYLO v26.33.02 ${asset}: checksum and machine protocol`);
}

await main();
