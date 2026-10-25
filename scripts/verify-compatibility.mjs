import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir, release } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

// Opt-in release verification, exclusively using virtual public fixtures and
// isolated directories. Preserve the stopped source and all run evidence.
const root = fileURLToPath(new URL('../', import.meta.url));
const require = createRequire(new URL('../frontend/package.json', import.meta.url));
const { chromium } = require('@playwright/test');
const binaryName = process.platform === 'win32' ? 'sunchang-hansang.exe' : 'sunchang-hansang';
const sourceBinary = join(root, 'target', 'release', binaryName);
const evidenceParent = join(root, 'tmp', 'verification', 'compatibility');
mkdirSync(evidenceParent, { recursive: true });
const evidence = mkdtempSync(join(evidenceParent, 'run-'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const fixtureBytes = readFileSync(join(root, 'tests/fixtures/snapshot-v2-schema14.json'));
const fixture = JSON.parse(fixtureBytes);
const report = {
  startedAt: new Date().toISOString(), platform: process.platform, arch: process.arch,
  osRelease: release(), node: process.version, binarySha256: hash(readFileSync(sourceBinary)),
  fixture: { schema: 14, sha256: hash(fixtureBytes), provenance: 'virtual data captured through the earlier implementation API; not an external release' },
  result: 'running', checks: {},
};
const apps = [];
let browser;

function files(directory, prefix = '') {
  return Object.fromEntries(readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const key = `${prefix}${entry.name}`, path = join(directory, entry.name);
    if (entry.isDirectory()) return Object.entries(files(path, `${key}/`));
    if (!entry.isFile() || entry.name === '.writer.lock') return [];
    return [[key, hash(readFileSync(path))]];
  }).sort(([a], [b]) => a.localeCompare(b)));
}
function immutableFiles(directory) {
  return Object.fromEntries(Object.entries(files(join(directory, 'db'))).filter(([path]) =>
    path.startsWith('snapshots/') || path.startsWith('imports/') || path.startsWith('assets/')));
}
function prepare(label) {
  const directory = join(evidence, label);
  mkdirSync(directory);
  assert.equal(hash(readFileSync(sourceBinary)), report.binarySha256, 'The release candidate changed during verification');
  cpSync(sourceBinary, join(directory, binaryName));
  return directory;
}
async function launch(directory, shouldStart = true) {
  const child = spawn(join(directory, binaryName), ['--no-browser'], {
    cwd: tmpdir(), env: { ...process.env, PATH: '' }, windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const app = { child, directory, stdout: '', stderr: '', closed: once(child, 'close') };
  apps.push(app);
  child.stdout.on('data', data => { app.stdout += String(data); });
  child.stderr.on('data', data => { app.stderr += String(data); });
  if (!shouldStart) {
    let timer;
    try {
      const [code, signal] = await Promise.race([
        app.closed,
        new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Expected startup refusal timed out')), 30_000); }),
      ]);
      assert.equal(code, 1); assert.equal(signal, null);
      assert.ok(!app.stdout.includes('#token='));
      return app;
    } finally { clearTimeout(timer); }
  }
  for (let n = 0; n < 300; n++) {
    const match = app.stdout.match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/);
    if (match) { app.url = match[0]; break; }
    assert.equal(child.exitCode, null, `App exited before serving: ${app.stderr}`);
    await delay(100);
  }
  assert.ok(app.url, `App did not start: ${app.stderr}`);
  app.page = await browser.newPage();
  await app.page.goto(app.url);
  await app.page.getByLabel('재료 이름', { exact: false }).waitFor({ state: 'visible' });
  await app.page.waitForFunction(() => !location.hash);
  return app;
}
async function request(app, path, data, expected = data === undefined ? 200 : 201) {
  const response = data === undefined
    ? await app.page.request.get(new URL(path, app.url).href)
    : await app.page.request.post(new URL(path, app.url).href, { headers: { Origin: new URL(app.url).origin }, data });
  assert.equal(response.status(), expected, `${path}: ${await response.text()}`);
  return response.json();
}
async function meta(app) {
  const state = await request(app, '/api/state');
  return { request_id: randomUUID(), epoch: state.epoch, expected_revision: state.revision };
}
async function material(app, name) {
  const input = { ...await meta(app), name, unit: 'g', kind: 'raw', description: '' };
  return { input, result: await request(app, '/api/materials', input) };
}
async function preview(app, target, expected = 200) {
  return request(app, '/api/restore/preview', { target_id: target.target_id, expected_sha256: target.sha256 }, expected);
}
async function restore(app, target) {
  assert.equal((await preview(app, target)).can_restore, true);
  const input = { meta: await meta(app), target_id: target.target_id, expected_sha256: target.sha256 };
  return { input, result: await request(app, '/api/restores', input) };
}
async function normalExit(app) {
  const started = performance.now();
  await app.page.goto('about:blank');
  const timer = setTimeout(() => { app.child.kill(); }, 150_000);
  try {
    const [code, signal] = await app.closed;
    assert.equal(signal, null, 'A signal exit is not normal termination evidence');
    assert.equal(code, 0);
    assert.ok(app.stdout.includes('순창한상을 안전하게 종료했습니다.'));
    const elapsedMs = Math.round(performance.now() - started);
    assert.ok(elapsedMs >= 119_000, 'The production grace must remain 120 seconds');
    return elapsedMs;
  } finally { clearTimeout(timer); await app.page.close(); }
}
async function observations(app) {
  const paths = ['/api/state', '/api/materials', '/api/overview', '/api/business', '/api/restore-targets?source=journal', '/api/restore-targets?source=import'];
  for (const [area, route] of [
    ['receipts', 'receipts'], ['productions', 'productions'], ['sales', 'sales'],
    ['stocktake_batches', 'stocktakes'], ['correction_records', 'corrections'],
    ['purchase_returns', 'purchase-returns'], ['business_records', 'business'],
  ]) for (const row of fixture.state[area]) paths.push(`/api/${route}/${row.id}`);
  for (const row of fixture.state.materials) paths.push(`/api/materials/${row.id}`, `/api/holdings/${row.id}`);
  for (const row of fixture.state.recipe_heads) paths.push(`/api/recipes/${row.material_id}`);
  for (const row of fixture.state.material_profile_heads) paths.push(`/api/materials/${row.material_id}/profile`);
  const results = {};
  for (const path of [...new Set(paths)]) results[path] = await request(app, path);
  return results;
}
function checkUnchanged(original, actual) {
  for (const [path, expected] of Object.entries(original)) assert.equal(actual[path], expected, path);
}

try {
  browser = await chromium.launch();
  report.browser = browser.version();
  const originalDirectory = prepare('이동 전 원본');
  const original = await launch(originalDirectory);
  const seeded = await material(original, '가상 이동 사진 준비');
  let profileVersion = null;
  for (const row of fixture.state.photo_assets) {
    const asset = row.descriptor;
    const bytes = readFileSync(join(root, `tests/fixtures/schema-v14-photo-${asset.sha256}.png`));
    const metadata = { meta: await meta(original), expected_material_version_id: seeded.result.version_id,
      expected_profile_version_id: profileVersion,
      photos: [{ source: 'upload', caption: '가상 호환 사진' }], primary_position: 0, suppliers: [] };
    const response = await original.page.request.post(new URL(`/api/materials/${seeded.result.id}/profiles`, original.url).href, {
      headers: { Origin: new URL(original.url).origin },
      multipart: { metadata: JSON.stringify(metadata), photo: { name: 'virtual.png', mimeType: 'image/png', buffer: bytes } },
    });
    assert.equal(response.status(), 201, await response.text());
    const result = await response.json(); profileVersion = result.version.id;
    assert.deepEqual(result.version.photos[0].asset, asset);
  }
  const importResponse = await original.page.request.post(new URL('/api/restore-targets/import', original.url).href, {
    headers: { Origin: new URL(original.url).origin, 'Content-Type': 'application/json' }, data: fixtureBytes,
  });
  assert.equal(importResponse.status(), 201, await importResponse.text());
  const imported = await importResponse.json();
  await restore(original, imported);
  const selected = (await request(original, '/api/restore-targets?source=journal')).rows[0];
  const later = await material(original, '가상 복원 후 보존 요청');
  const restored = await restore(original, selected);
  const staleBefore = await request(original, '/api/materials', later.input, 409);
  assert.equal(staleBefore.code, 'stale_generation');
  assert.equal(staleBefore.request_status, 'already_processed');
  const conflictBefore = await request(original, '/api/materials', { ...later.input, name: '가상 다른 본문' }, 409);
  assert.equal(conflictBefore.code, 'request_conflict');
  const before = await observations(original);
  const beforeFiles = immutableFiles(originalDirectory);
  writeFileSync(join(evidence, 'before-api.json'), JSON.stringify(before, null, 2));
  report.checks.firstNormalExitMs = await normalExit(original);
  const stoppedSource = files(join(originalDirectory, 'db'));

  const movedDirectory = join(evidence, '한글 공백 이동 경로');
  // Move the stopped complete application folder, then replace only its binary
  // with the reviewed compatible candidate. This is same-OS, not cross-OS.
  renameSync(originalDirectory, movedDirectory);
  assert.equal(hash(readFileSync(sourceBinary)), report.binarySha256, 'The release candidate changed during verification');
  cpSync(sourceBinary, join(movedDirectory, binaryName));
  assert.deepEqual(files(join(movedDirectory, 'db')), stoppedSource);
  const moved = await launch(movedDirectory);
  assert.deepEqual(await observations(moved), before);
  checkUnchanged(beforeFiles, immutableFiles(movedDirectory));
  assert.deepEqual(await request(moved, '/api/materials', later.input, 409), staleBefore);
  assert.deepEqual(await request(moved, '/api/materials', { ...later.input, name: '가상 다른 본문' }, 409), conflictBefore);
  assert.deepEqual(await request(moved, '/api/restores', restored.input), restored.result);
  for (const row of fixture.state.photo_assets) {
    const response = await moved.page.request.get(new URL(`/api/photo-assets/${row.descriptor.sha256}`, moved.url).href);
    assert.equal(response.status(), 200);
    assert.equal(hash(await response.body()), row.descriptor.sha256);
  }
  assert.equal((await preview(moved, imported)).can_restore, true);
  const beforeRestore = restored.result.previous_target;
  const recovered = await restore(moved, beforeRestore);
  assert.ok((await request(moved, '/api/materials')).some(item => item.id === later.result.id));
  assert.deepEqual(await request(moved, '/api/restores', recovered.input), recovered.result);
  checkUnchanged(beforeFiles, immutableFiles(movedDirectory));
  report.checks.movedApiComparisons = Object.keys(before).length;
  report.checks.movedImmutableFiles = Object.keys(beforeFiles).length;
  report.checks.repeatedRestoreAndPreservedRequests = 'pass';
  report.checks.secondNormalExitMs = await normalExit(moved);

  const fullStoppedFiles = files(join(movedDirectory, 'db'));
  const dbOnlyDirectory = prepare('DB만 복사');
  mkdirSync(join(dbOnlyDirectory, 'db'));
  cpSync(join(movedDirectory, 'db/app.sqlite3'), join(dbOnlyDirectory, 'db/app.sqlite3'));
  const missingPhotos = await launch(dbOnlyDirectory, false);
  assert.match(missingPhotos.stderr, /사진/);
  const sourceSnapshots = files(join(movedDirectory, 'db/snapshots'));
  assert.deepEqual(files(join(dbOnlyDirectory, 'db/snapshots')), sourceSnapshots);
  assert.ok(!existsSync(join(dbOnlyDirectory, 'db/imports')) || readdirSync(join(dbOnlyDirectory, 'db/imports')).length === 0);
  report.checks.dbOnly = { outboxFilesRecovered: Object.keys(sourceSnapshots).length, missingPhotosRefused: true, importedOriginalsRecovered: false };

  // Restoring only the missing photos permits startup, but it cannot reconstruct
  // imported JSON originals or the before-restore archive from metadata alone.
  cpSync(join(movedDirectory, 'db/assets'), join(dbOnlyDirectory, 'db/assets'), { recursive: true });
  const importsMissing = await launch(dbOnlyDirectory);
  const incompleteState = await request(importsMissing, '/api/state');
  const failedImport = await preview(importsMissing, imported, 503);
  assert.equal(failedImport.code, 'unavailable');
  const failedBefore = await preview(importsMissing, beforeRestore, 503);
  assert.equal(failedBefore.code, 'unavailable');
  assert.deepEqual(await request(importsMissing, '/api/state'), incompleteState);
  report.checks.dbOnly.importAndBeforeRestoreMissingRefused = true;
  report.checks.thirdNormalExitMs = await normalExit(importsMissing);
  assert.deepEqual(files(join(movedDirectory, 'db')), fullStoppedFiles, 'The full stopped source must remain untouched');
  report.result = 'pass';
} catch (error) {
  report.result = 'fail'; report.error = String(error?.stack ?? error); process.exitCode = 1;
} finally {
  for (const app of apps) if (app.child.exitCode === null && app.child.signalCode === null) { app.child.kill(); await app.closed; }
  await browser?.close();
  report.finishedAt = new Date().toISOString();
  writeFileSync(join(evidence, 'result.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ evidence: relative(root, evidence), ...report }, null, 2));
}
