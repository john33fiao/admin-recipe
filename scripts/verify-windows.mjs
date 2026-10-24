import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

// Opt-in real-time verification. Only processes and synthetic data created here
// are touched. Evidence stays in the repository's ignored tmp/ directory.
assert.equal(process.platform, 'win32', 'This verification requires Windows.');
const root = fileURLToPath(new URL('../', import.meta.url));
const evidenceRoot = join(root, 'tmp', 'verification', 'windows-runtime');
mkdirSync(evidenceRoot, { recursive: true });
const evidence = mkdtempSync(join(evidenceRoot, 'run-'));
console.log(`Evidence: ${relative(root, evidence)}`);
const binarySha256 = createHash('sha256').update(readFileSync(join(root, 'target/release/sunchang-hansang.exe'))).digest('hex');
const duration = 30 * 60_000;

async function connectCdp(endpoint) {
  const socket = new WebSocket(endpoint);
  await once(socket, 'open');
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener('message', event => {
    const response = JSON.parse(event.data);
    const callback = pending.get(response.id);
    if (callback) { pending.delete(response.id); callback(response); }
  });
  return {
    close: () => socket.close(),
    send: (method, params = {}, sessionId) => new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15_000);
      pending.set(id, response => {
        clearTimeout(timer);
        if (response.error) reject(new Error(response.error.message));
        else resolve(response.result);
      });
      socket.send(JSON.stringify({ id, method, params, sessionId }));
    }),
  };
}

function launch(label) {
  const directory = mkdtempSync(join(evidence, `${label}-한글 공백 `));
  const executable = join(directory, 'sunchang-hansang.exe');
  copyFileSync(join(root, 'target', 'release', 'sunchang-hansang.exe'), executable);
  const child = spawn(executable, ['--no-browser'], {
    cwd: tmpdir(), env: { ...process.env, PATH: '' }, windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', chunk => { stdout += String(chunk); });
  child.stderr.on('data', chunk => { stderr += String(chunk); });
  const exited = once(child, 'exit');
  return { child, directory, executable, exited, stdout: () => stdout, stderr: () => stderr };
}

async function address(app) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const match = app.stdout().match(/http:\/\/127\.0\.0\.1:\d+\/#token=[a-f0-9]+/);
    if (match) return match[0];
    assert.equal(app.child.exitCode, null, `App exited before serving: ${app.stderr()}`);
    await delay(100);
  }
  throw new Error('App did not announce its address.');
}

async function normalExit(app, maximumMs = 195_000) {
  let timer;
  try {
    const [code] = await Promise.race([
      app.exited,
      new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Graceful exit timed out.')), maximumMs); }),
    ]);
    assert.equal(code, 0, 'App must exit successfully without being killed.');
    assert.ok(app.stdout().includes('순창한상을 안전하게 종료했습니다.'));
  } finally {
    clearTimeout(timer);
  }
}

async function cleanup(app) {
  if (app.child.exitCode === null && app.child.signalCode === null) {
    app.child.kill();
    await app.exited;
  }
}

async function noClient() {
  const app = launch('initial');
  const started = performance.now();
  try {
    await address(app);
    await normalExit(app, 150_000);
    const elapsedMs = Math.round(performance.now() - started);
    assert.ok(elapsedMs >= 119_000, 'Initial grace must not be shortened.');
    return { scenario: 'initial-no-client', binarySha256, elapsedMs, result: 'pass' };
  } finally {
    await cleanup(app);
  }
}

async function browserRun(channel) {
  const app = launch(channel);
  const record = { scenario: channel, binarySha256, result: 'running', startedAt: new Date().toISOString(), samples: [] };
  let cdp;
  let browserProcess;
  try {
    const url = await address(app);
    const browserExecutable = channel === 'chrome'
      ? join(process.env.ProgramFiles, 'Google', 'Chrome', 'Application', 'chrome.exe')
      : join(process.env['ProgramFiles(x86)'], 'Microsoft', 'Edge', 'Application', 'msedge.exe');
    const profile = join(app.directory, 'browser-profile');
    // Use a dedicated profile and normal browser defaults. Playwright's normal
    // launch forces focus and disables background throttling, invalidating this test.
    browserProcess = spawn(browserExecutable, [
      `--user-data-dir=${profile}`, '--remote-debugging-port=0',
      '--no-first-run', '--no-default-browser-check', 'about:blank',
    ], { stdio: 'ignore' });
    let endpoint;
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      try {
        const [port, path] = readFileSync(join(profile, 'DevToolsActivePort'), 'utf8').trim().split('\n');
        endpoint = `ws://127.0.0.1:${Number(port)}${path}`;
      } catch { /* Browser starting. */ }
      if (endpoint) break;
      await delay(100);
    }
    assert.ok(endpoint, 'Dedicated browser did not expose its debugging port.');
    // Raw CDP avoids Playwright's focus/viewport emulation. Actual document
    // visibility must be hidden; no mock clock or visibility override is used.
    cdp = await connectCdp(endpoint);
    record.browserVersion = (await cdp.send('Browser.getVersion')).product;
    const { processInfo } = await cdp.send('SystemInfo.getProcessInfo');
    const browserPid = processInfo.find(info => info.type === 'browser')?.id;
    assert.ok(Number.isInteger(browserPid) && browserPid > 0);
    const { targetInfos } = await cdp.send('Target.getTargets');
    const { targetId } = targetInfos.find(target => target.type === 'page');
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const evaluate = async expression => {
      const result = await cdp.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
      assert.ok(!result.exceptionDetails, 'Browser evaluation failed.');
      return result.result.value;
    };
    const waitFor = async expression => {
      const deadline = Date.now() + 20_000;
      while (Date.now() < deadline) {
        if (await evaluate(expression)) return;
        await delay(200);
      }
      throw new Error(`Browser state did not become ready: ${expression}`);
    };
    const ready = "!!document.querySelector('input') && !document.querySelector('input').disabled && !location.hash";
    await cdp.send('Page.navigate', { url }, sessionId);
    await waitFor(ready);
    const material = await evaluate(`fetch('/api/materials', { method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({request_id:crypto.randomUUID(),name:'가상 ${channel} 장시간',unit:'g'}) }).then(async r => ({status:r.status,body:await r.json()}))`);
    assert.equal(material.status, 201);
    record.materialId = material.body.id;
    await cdp.send('Page.navigate', { url: new URL(`/materials/${material.body.id}`, url).toString() }, sessionId);
    await waitFor(`Array.from(document.querySelectorAll('h1,h2')).some(h => h.textContent.includes('가상 ${channel} 장시간'))`);
    await cdp.send('Page.reload', {}, sessionId);
    await waitFor(`Array.from(document.querySelectorAll('h1,h2')).some(h => h.textContent.includes('가상 ${channel} 장시간'))`);
    const cover = await cdp.send('Target.createTarget', { url: 'about:blank', newWindow: false });
    await cdp.send('Target.activateTarget', { targetId: cover.targetId });
    const { windowId } = await cdp.send('Browser.getWindowForTarget', { targetId });
    await cdp.send('Browser.setWindowBounds', { windowId, bounds: { windowState: 'minimized' } });
    await waitFor("document.visibilityState === 'hidden'");
    const start = performance.now();
    console.log(`${channel}: ${record.browserVersion}; background observation started`);
    while (performance.now() - start < duration) {
      await delay(Math.min(60_000, duration - (performance.now() - start)));
      assert.equal(app.child.exitCode, null, 'App exited while browser heartbeat was live.');
      assert.equal(await evaluate('document.visibilityState'), 'hidden');
      record.samples.push({ elapsedMs: Math.round(performance.now() - start), at: new Date().toISOString(), hidden: true, running: true });
      console.log(`${channel}: background ${Math.round((performance.now() - start) / 60_000)} min`);
    }
    record.backgroundMs = Math.round(performance.now() - start);
    await cdp.send('Browser.setWindowBounds', { windowId, bounds: { windowState: 'normal' } });
    await cdp.send('Target.activateTarget', { targetId });
    await cdp.send('Page.navigate', { url: new URL('/', url).toString() }, sessionId);
    await waitFor(ready);
    const saved = await evaluate(`fetch('/api/materials/${material.body.id}').then(r=>r.json())`);
    assert.equal(saved.id, material.body.id);
    assert.equal(saved.unit, 'g');
    const screenshot = await cdp.send('Page.captureScreenshot', {}, sessionId);
    writeFileSync(join(app.directory, 'after-background.png'), Buffer.from(screenshot.data, 'base64'));
    const killedAt = performance.now();
    // This PID came from this dedicated browser's CDP session. Never kill an
    // existing user browser by process name or executable path.
    process.kill(browserPid);
    await normalExit(app);
    record.exitAfterBrowserKillMs = Math.round(performance.now() - killedAt);
    assert.ok(record.exitAfterBrowserKillMs >= 119_000);
    record.result = 'pass';
    return record;
  } catch (error) {
    record.result = 'fail';
    record.error = String(error).replace(/#token=[a-f0-9]+/g, '#token=[redacted]');
    throw error;
  } finally {
    cdp?.close();
    if (browserProcess && browserProcess.exitCode === null && browserProcess.signalCode === null) browserProcess.kill();
    await cleanup(app);
    writeFileSync(join(evidence, `${channel}.json`), JSON.stringify(record, null, 2) + '\n');
  }
}

const results = await Promise.allSettled([noClient(), browserRun('chrome'), browserRun('msedge')]);
const summary = results.map((result, index) => result.status === 'fulfilled'
  ? result.value : { scenario: ['initial-no-client', 'chrome', 'msedge'][index], result: 'fail', error: String(result.reason).replace(/#token=[a-f0-9]+/g, '#token=[redacted]') });
writeFileSync(join(evidence, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
console.log(JSON.stringify(summary.map(({ samples, ...result }) => result), null, 2));
if (results.some(result => result.status === 'rejected')) process.exitCode = 1;
