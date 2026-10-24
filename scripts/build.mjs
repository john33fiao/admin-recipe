import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { writeManifest } from './ui-manifest.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
function run(command, args, cwd) {
  const result = spawnSync(command, args, {cwd, stdio:'inherit', shell:process.platform === 'win32', env:{...process.env, NUXT_TELEMETRY_DISABLED:'1'}});
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run(npm, ['run', 'generate'], `${root}/frontend`);
writeManifest(root);
run('cargo', ['build', '--locked', '--release'], root);
