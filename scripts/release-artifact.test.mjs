import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

test('release executable contains no build workspace or user home paths', () => {
  const root = fileURLToPath(new URL('../', import.meta.url));
  const binary = readFileSync(resolve(root, 'target/release', process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang'));
  for (const path of [resolve(root), homedir()]) {
    for (const variant of new Set([path, path.replaceAll('\\', '/'), path.replaceAll('/', '\\')])) {
      for (const encoding of ['utf8', 'utf16le']) {
        assert.equal(binary.includes(Buffer.from(variant, encoding)), false, `Release contains a local build path (${encoding}).`);
      }
    }
  }
});
