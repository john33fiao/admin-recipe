import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { writeManifest, verifyManifest } from './ui-manifest.mjs';

test('reject missing output, changed source, altered assets and removed chunks', () => {
  const root = mkdtempSync(join(tmpdir(), 'hansang-build-'));
  try {
    mkdirSync(join(root, 'frontend/app'), {recursive:true});
    writeFileSync(join(root, 'frontend/app/app.vue'), '<template>test</template>');
    assert.throws(() => writeManifest(root), /index.html/);
    mkdirSync(join(root, 'frontend/.output/public/_nuxt'), {recursive:true});
    writeFileSync(join(root, 'frontend/.output/public/index.html'), '<script src="/_nuxt/app.js"></script>');
    writeFileSync(join(root, 'frontend/.output/public/_nuxt/app.js'), 'hello');
    writeManifest(root); verifyManifest(root);
    writeFileSync(join(root, 'frontend/app/app.vue'), '<template>new source</template>');
    assert.throws(() => verifyManifest(root), /입력/);
    writeManifest(root); verifyManifest(root);
    writeFileSync(join(root, 'frontend/.output/public/_nuxt/app.js'), 'tampered');
    assert.throws(() => verifyManifest(root), /산출물/);
    writeManifest(root);
    rmSync(join(root, 'frontend/.output/public/_nuxt/app.js'));
    assert.throws(() => verifyManifest(root), /산출물/);
  } finally { rmSync(root, { recursive:true, force:true }); }
});
