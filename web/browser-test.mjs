import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir } from 'node:fs/promises';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const origin = process.env.GAME_URL || 'http://127.0.0.1:8787';
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, reducedMotion: 'reduce' });
const apiRequests = [];
context.on('request', request => { if (new URL(request.url()).pathname.includes('/api/')) apiRequests.push(request.url()); });
await context.addInitScript(() => {
  const NativeWorker = window.Worker;
  window.engineStats = { searches: 0, terminated: 0 };
  window.Worker = class extends NativeWorker {
    postMessage(message, ...args) {
      if (message.action === 'ai') window.engineStats.searches++;
      super.postMessage(message, ...args);
    }
    terminate() { window.engineStats.terminated++; super.terminate(); }
  };
});
const page = await context.newPage();
const errors = [];
page.on('pageerror', e => errors.push(e.message));
const ready = () => page.waitForFunction(() => document.body.dataset.busy === 'false' && document.querySelectorAll('.square').length === 64);
const count = n => page.waitForFunction(n => document.querySelector('#move-count').textContent === String(n) && document.body.dataset.busy === 'false', n);
const square = s => page.locator(`.square[data-square="${s}"]`);
async function startDrag(from, to) {
  const source = await square(from).boundingBox();
  const target = typeof to === 'string' ? await square(to).boundingBox() : to;
  await page.mouse.move(source.x + source.width / 2, source.y + source.height / 2);
  await page.mouse.down();
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 8 });
}
async function drag(from, to) {
  await startDrag(from, to);
  await page.mouse.up();
}
async function quickDrag(from, to) {
  const source = await square(from).boundingBox();
  const target = typeof to === 'string' ? await square(to).boundingBox() : to;
  const input = await context.newCDPSession(page);
  try {
    // A fast gesture may reach pointerup before any pointermove is delivered.
    await input.send('Input.dispatchMouseEvent', { type: 'mousePressed',
      x: source.x + source.width / 2, y: source.y + source.height / 2,
      button: 'left', buttons: 1, clickCount: 1 });
    await input.send('Input.dispatchMouseEvent', { type: 'mouseReleased',
      x: target.x + target.width / 2, y: target.y + target.height / 2,
      button: 'left', buttons: 0, clickCount: 1 });
  } finally { await input.detach(); }
}
async function fixture(fen, human = 'white') {
  await page.evaluate(({ fen, human }) => localStorage.setItem('african-chess:play:v1', JSON.stringify({ version: 1, fen, human, level: 'relaxed', moves: [], flipped: false })), { fen, human });
  await page.reload(); await ready();
}
try {
  await page.goto(origin); await ready();
  assert.equal(await page.locator('.board .piece').count(), 32);
  assert.equal(await page.locator('.square').first().getAttribute('data-square'), 'h1');
  await quickDrag('e7', 'e5');
  await count(2);
  await page.locator('#undo').click(); await count(0);
  await quickDrag('e7', 'e4');
  assert.equal(await page.locator('#move-count').textContent(), '0');
  assert.equal(await square('e7').getAttribute('data-piece'), 'pawn');
  await quickDrag('e7', { x: 0, y: 0, width: 1, height: 1 });
  assert.equal(await page.locator('#move-count').textContent(), '0');
  await drag('e7', 'e5'); await count(2);
  await page.locator('#undo').click(); await count(0);
  await drag('e7', 'e4');
  assert.equal(await page.locator('#move-count').textContent(), '0');
  assert.equal(await square('e7').getAttribute('data-piece'), 'pawn');
  await drag('e7', { x: 0, y: 0, width: 1, height: 1 });
  assert.equal(await page.locator('#move-count').textContent(), '0');
  await drag('e2', 'e4');
  assert.equal(await page.locator('#move-count').textContent(), '0');
  await page.evaluate(() => {
    window.dragCaptureId = null;
    document.querySelector('#board').addEventListener('gotpointercapture', event => {
      window.dragCaptureId = event.pointerId;
    }, { once: true });
  });
  await startDrag('e7', 'e5');
  assert.ok((await square('e5').getAttribute('class')).includes('drop-target'));
  assert.equal(await page.evaluate(() => {
    const board = document.querySelector('#board');
    if (!board.hasPointerCapture(window.dragCaptureId)) return false;
    board.releasePointerCapture(window.dragCaptureId);
    return true;
  }), true, 'the highlighted drag had capture before losing it');
  await page.mouse.up(); await count(2);
  await page.locator('#undo').click(); await count(0);
  await startDrag('e7', 'e5');
  assert.ok((await square('e5').getAttribute('class')).includes('drop-target'));
  assert.equal(await page.locator('.drag-preview').count(), 1);
  await page.keyboard.press('Escape'); await page.mouse.up();
  assert.equal(await page.locator('#move-count').textContent(), '0');
  assert.equal(await page.locator('.drag-preview').count(), 0);
  await startDrag('e7', 'e5');
  await page.evaluate(() => document.querySelector('#new-game').click());
  await page.mouse.up(); await count(0);
  assert.equal(await page.locator('.drag-preview').count(), 0);
  await page.locator('#flip').click();
  await drag('d7', 'd5'); await count(2);
  await page.locator('#undo').click(); await count(0);
  await page.locator('#flip').click();
  await square('e7').click();
  assert.ok((await square('e5').getAttribute('class')).includes('legal'));
  await square('e5').click(); await count(2);
  assert.equal(await page.locator('body').getAttribute('data-turn'), 'black');
  await page.reload(); await count(2);
  await page.locator('#undo').click(); await count(0);
  await page.locator('[data-side="white"]').click(); await count(1);
  assert.equal(await page.locator('body').getAttribute('data-turn'), 'white');
  assert.equal(await page.locator('#undo').isDisabled(), true);
  await page.locator('#flip').click();
  assert.equal(await page.locator('.square').first().getAttribute('data-square'), 'h1');
  await page.locator('[data-side="black"]').click(); await count(0);
  await page.locator('#strength').selectOption('challenging');
  const beforeReset = await page.evaluate(() => ({ ...window.engineStats }));
  await square('e7').click(); await square('e5').click();
  await page.waitForFunction(n => window.engineStats.searches > n, beforeReset.searches);
  await page.locator('#new-game').click(); await count(0);
  assert.ok(await page.evaluate(n => window.engineStats.terminated > n, beforeReset.terminated), 'reset must terminate a running search');
  await page.waitForTimeout(600); assert.equal(await page.locator('#move-count').textContent(), '0');
  const beforeUndo = await page.evaluate(() => ({ ...window.engineStats }));
  await square('e7').click(); await square('e5').click();
  await page.waitForFunction(n => window.engineStats.searches > n, beforeUndo.searches);
  await page.locator('#undo').click(); await count(0);
  assert.ok(await page.evaluate(n => window.engineStats.terminated > n, beforeUndo.terminated), 'undo must terminate a running search');
  // Control positions for the reported f6 giraffe: jumps work unless they
  // expose the king; a stationary capture can still be legal while pinned.
  await fixture('4k3/5pp1/5n2/6QR/8/8/8/K7 b - - 0 1', 'black');
  await square('f6').click();
  assert.equal(await page.locator('.square.legal').count(), 7);
  assert.ok((await square('h5').getAttribute('class')).includes('capture-target'));
  await fixture('3k4/5pp1/5n2/6QR/8/8/8/K7 b - - 0 1', 'black');
  await square('f6').click();
  assert.equal(await page.locator('.square.legal').count(), 0);
  assert.match(await page.locator('#status-detail').textContent(), /no legal moves/);
  await fixture('3k4/5pp1/5n2/6QR/8/8/5P2/K7 b - - 0 1', 'black');
  await square('f6').click();
  assert.equal(await page.locator('.square.legal').count(), 1);
  assert.ok((await square('f2').getAttribute('class')).includes('stationary-target'));
  await fixture('7k/8/1q6/8/8/8/1N6/K7 w - - 0 1');
  await square('b2').click();
  assert.ok((await square('b6').getAttribute('class')).includes('stationary-target'));
  assert.equal(await square('b6').evaluate(element => parseFloat(getComputedStyle(element, '::after').width) > element.clientWidth * 0.65), true, 'capture ring should surround the target');
  await drag('b2', 'b6'); await count(2);
  assert.equal(await square('b2').getAttribute('data-piece'), 'giraffe');
  assert.equal(await square('b6').getAttribute('data-piece'), '');
  await fixture('7k/8/8/8/3bB3/8/8/K2Q4 w - - 0 1');
  assert.ok((await square('e4').getAttribute('class')).includes('frozen'));
  await square('e4').click();
  assert.match(await page.locator('#status-detail').textContent(), /frozen/i);
  await drag('e4', 'f5');
  assert.equal(await page.locator('#move-count').textContent(), '0');
  await square('d1').click(); await square('d4').click(); await count(2);
  assert.ok(!(await square('e4').getAttribute('class')).includes('frozen'));
  await fixture('7k/P7/8/8/8/8/8/7K w - - 0 1');
  await drag('a7', 'a8');
  await page.locator('#promotion[open]').waitFor();
  assert.equal(await page.locator('#promotion-options button').count(), 4);
  await page.getByRole('button', { name: 'Promote to giraffe' }).click(); await count(2);
  assert.equal(await square('a8').getAttribute('data-piece'), 'giraffe');
  await fixture('7k/6Q1/5K2/8/8/8/8/8 b - - 0 1', 'white');
  assert.equal(await page.locator('#status-title').textContent(), 'You win.');
  assert.equal(await page.locator('.square.legal').count(), 0);
  await page.locator('[data-side="black"]').click(); await count(0);
  await context.route('**/*.wasm', route => route.abort());
  await page.reload(); await page.locator('#error:not([hidden])').waitFor();
  await context.unroute('**/*.wasm');
  await page.locator('#retry').click(); await count(0);
  assert.equal(await page.locator('#error').isHidden(), true);
  // After loading, moves and AI require no network, including a new game.
  await page.locator('#strength').selectOption('relaxed');
  await context.setOffline(true);
  await square('e7').click(); await square('e5').click(); await count(2);
  await page.locator('#undo').click(); await count(0);
  await page.locator('#new-game').click(); await count(0);
  await square('d7').click(); await square('d5').click(); await count(2);
  await context.setOffline(false);
  await mkdir(new URL('./artifacts/', import.meta.url), { recursive: true });
  await page.screenshot({ path: new URL('./artifacts/desktop.png', import.meta.url).pathname, fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await fixture(null, 'black');
  const touch = await context.newCDPSession(page);
  const source = await square('e7').boundingBox();
  const target = await square('e5').boundingBox();
  const point = box => ({ x: box.x + box.width / 2, y: box.y + box.height / 2 });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [point(source)] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [point(target)] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] });
  assert.equal(await page.locator('#move-count').textContent(), '0');
  await page.locator('#flip').evaluate(button => button.focus({ preventScroll: true }));
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [point(source)] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [point(target)] });
  assert.equal(await page.locator('.drag-preview').count(), 1);
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('.drag-preview').count(), 0, 'Escape cancels a touch drag even when focus is outside the board');
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  assert.equal(await page.locator('#move-count').textContent(), '0');
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [point(source)] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [point(target)] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await count(2);
  await page.locator('#undo').click(); await count(0);
  await square('e7').scrollIntoViewIfNeeded();
  for (const name of ['e7', 'e5']) {
    const box = await square(name).boundingBox();
    await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [point(box)] });
    await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  }
  await count(2);
  await touch.detach();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.screenshot({ path: new URL('./artifacts/mobile.png', import.meta.url).pathname, fullPage: true });
  assert.deepEqual(errors, []);
  assert.deepEqual(apiRequests, [], 'gameplay must never call an API');
  console.log('Browser checks passed: mouse and touch dragging, invalid drops, touch cancellation, flipped dragging, moves, WASM AI, save/restore, undo, sides, flip, active search cancellation, giraffe pins and highlights, stationary capture, freeze/thaw, promotion, checkmate, WASM retry, offline play, zero API requests, mobile layout.');
} finally { await browser.close(); }
