import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const skipped = new Set(['dist', 'node_modules', '.nuxt', '.output', '.cache', 'test-results', 'playwright-report']);
function tree(root, input = false, prefix = '') {
  const result = {};
  for (const entry of readdirSync(join(root, prefix), {withFileTypes:true}).sort((a,b) => a.name.localeCompare(b.name, 'en'))) {
    if (input && (skipped.has(entry.name) || entry.name === '.DS_Store' || entry.name.startsWith('.env') || entry.name.endsWith('.log'))) continue;
    const path = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isSymbolicLink()) throw new Error(`심볼릭 링크 자산은 허용하지 않습니다: ${path}`);
    if (entry.isDirectory()) Object.assign(result, tree(root, input, path));
    else result[path] = createHash('sha256').update(readFileSync(join(root, path))).digest('hex');
  }
  return result;
}
function capture(root) {
  const frontend = join(root, 'frontend');
  const output = join(frontend, '.output/public');
  if (!existsSync(join(output, 'index.html'))) throw new Error('index.html 없음: 먼저 Nuxt generate를 실행하세요.');
  return {version:1, inputs:tree(frontend, true), outputs:tree(output)};
}
export function writeManifest(root) {
  writeFileSync(join(root, 'frontend/.output/ui-manifest.json'), JSON.stringify(capture(root), null, 2) + '\n');
}
export function verifyManifest(root) {
  const old = JSON.parse(readFileSync(join(root, 'frontend/.output/ui-manifest.json'), 'utf8'));
  const current = capture(root);
  for (const [key, label] of [['inputs', '입력'], ['outputs', '산출물']]) {
    if (JSON.stringify(old[key]) !== JSON.stringify(current[key])) throw new Error(`UI ${label} 변경: 다시 생성하세요.`);
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = fileURLToPath(new URL('../', import.meta.url));
  if (process.argv[2] === '--check') verifyManifest(root);
  else if (process.argv[2] === '--write') writeManifest(root);
  else throw new Error('사용법: node scripts/ui-manifest.mjs --write | --check');
}
