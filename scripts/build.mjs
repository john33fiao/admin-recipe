import { spawnSync } from 'node:child_process';
import { homedir } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { writeManifest } from './ui-manifest.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
function run(command, args, cwd) {
  const result = spawnSync(command, args, {cwd, stdio:'inherit', shell:command === 'npm.cmd', env:{...process.env, NUXT_TELEMETRY_DISABLED:'1'}});
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run(npm, ['run', 'generate'], `${root}/frontend`);
writeManifest(root);
// Cargo merges matching target flags, preserving the MSVC static CRT settings.
// Pass these dynamic paths as arguments, never through a shell or public file.
const remaps = [[homedir(), '/build-home'], [resolve(root), '/source']].flatMap(([from, to]) =>
  [...new Set([from, from.replaceAll('\\', '/')])].map(path => `--remap-path-prefix=${path}=${to}`));
const config = `target.'cfg(all())'.rustflags = ${JSON.stringify(remaps)}`;
run('cargo', ['--config', config, 'build', '--locked', '--release'], root);
run(process.execPath, ['--test', 'scripts/release-artifact.test.mjs'], root);
