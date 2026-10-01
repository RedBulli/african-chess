import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const origin = process.env.GAME_URL || 'http://127.0.0.1:8787';
const binary = new URL('../target/release/web-engine', import.meta.url).pathname;
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.goto(origin);
  await page.evaluate(() => {
    window.testWorker = new Worker(new URL('./engine-worker.js', location.href), { type: 'module' });
    window.testSequence = 0;
  });
  const request = (action, payload) => page.evaluate(({ action, payload }) => new Promise((resolve, reject) => {
    const id = ++window.testSequence;
    const timer = setTimeout(() => reject(new Error('WASM test request timed out')), 30000);
    window.testWorker.onmessage = ({ data }) => { if (data.id === id) { clearTimeout(timer); resolve(data); } };
    window.testWorker.onerror = event => { clearTimeout(timer); reject(new Error(event.message)); };
    window.testWorker.postMessage({ id, action, payload });
  }), { action, payload });
  const cases = [
    ['state', {}], ['state', { moves: ['e7e5'] }],
    ['state', { fen: '7k/8/1q6/8/8/8/1N6/K7 w - - 0 1', moves: ['b2b6@'] }],
    ['state', { fen: '7k/P7/8/8/8/8/8/7K w - - 0 1', moves: ['a7a8n'] }],
    ['state', { fen: '7k/6Q1/5K2/8/8/8/8/8 b - - 0 1' }],
    ...['relaxed', 'balanced', 'challenging'].map(level => ['ai', { moves: ['e7e5'], human: 'black', level }]),
    ['ai', { human: 'white', level: 'relaxed' }],
  ];
  for (const [action, payload] of cases) {
    const args = [action, '--moves', (payload.moves || []).join(' '), '--human', payload.human || 'black', '--level', payload.level || 'balanced'];
    if (payload.fen) args.push('--fen', payload.fen);
    const native = JSON.parse(execFileSync(binary, args, { encoding: 'utf8', timeout: 30000 }));
    const wasm = await request(action, payload);
    assert.equal(wasm.error, undefined);
    assert.deepEqual(wasm.result, native, `${action} ${JSON.stringify(payload)}`);
  }
  for (const payload of [null, [], { moves: 'a7a6' }, { moves: [123] }, { moves: ['e2e4'] },
    { moves: Array(2001).fill('a7a6') }, { human: 'red' }, { level: 'bad' }, { fen: 'x'.repeat(201) }]) {
    assert.equal(typeof (await request('state', payload)).error, 'string');
  }
  assert.equal(typeof (await request('unknown', {})).error, 'string');
  assert.equal((await request('state', {})).result.pieces.length, 32, 'invalid requests must not poison the WASM instance');
  await page.evaluate(() => window.testWorker.terminate());
  console.log(`WASM/native parity passed: ${cases.length} positions/searches, all strengths, validation, and recovery.`);
} finally { await browser.close(); }
