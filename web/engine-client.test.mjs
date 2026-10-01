import assert from 'node:assert/strict';
import test from 'node:test';
import { EngineClient } from './engine-client.js';

class ControlledWorker extends EventTarget {
  messages = [];
  terminated = false;
  postMessage(message) { this.messages.push(message); }
  terminate() { this.terminated = true; }
  reply(data) { this.dispatchEvent(new MessageEvent('message', { data })); }
  fail() { this.dispatchEvent(new Event('error')); }
}
function setup(timeout = 1000) {
  const workers = [];
  const client = new EngineClient({
    createWorker: () => { const worker = new ControlledWorker(); workers.push(worker); return worker; },
    timeout,
  });
  return { client, workers };
}

test('reuses an idle worker and ignores unrelated replies', async () => {
  const { client, workers } = setup();
  const first = client.request('state', { moves: [] });
  const [{ id, action, payload }] = workers[0].messages;
  assert.equal(action, 'state');
  assert.deepEqual(payload, { moves: [] });
  workers[0].reply({ id: id + 1, result: 'stale' });
  workers[0].reply({ id, result: { turn: 'black' } });
  assert.deepEqual(await first, { turn: 'black' });
  const second = client.request('state', {});
  assert.equal(workers.length, 1);
  workers[0].reply({ id: workers[0].messages[1].id, result: {} });
  await second;
  client.dispose();
});

test('abort stops the worker and a stale result cannot overwrite the next request', async () => {
  const { client, workers } = setup();
  const abort = new AbortController();
  const first = client.request('ai', {}, abort.signal);
  const rejection = assert.rejects(first, { name: 'AbortError' });
  abort.abort();
  await rejection;
  assert.equal(workers[0].terminated, true);
  const second = client.request('state', {});
  assert.equal(workers.length, 2);
  workers[0].reply({ id: workers[0].messages[0].id, result: 'stale' });
  workers[1].reply({ id: workers[1].messages[0].id, result: 'current' });
  assert.equal(await second, 'current');
  client.dispose();
});

test('aborting a completed request preserves the idle worker', async () => {
  const { client, workers } = setup();
  const abort = new AbortController();
  const first = client.request('state', {}, abort.signal);
  workers[0].reply({ id: workers[0].messages[0].id, result: {} });
  await first;
  abort.abort();
  assert.equal(workers[0].terminated, false);
  client.dispose();
});

test('a request aborted before starting does not create a worker', async () => {
  const { client, workers } = setup();
  await assert.rejects(client.request('state', {}, AbortSignal.abort()), { name: 'AbortError' });
  assert.equal(workers.length, 0);
});

test('timeout stops search and permits a fresh request', async () => {
  const { client, workers } = setup(10);
  await assert.rejects(client.request('ai', {}), /too long/);
  assert.equal(workers[0].terminated, true);
  const retry = client.request('state', {});
  workers[1].reply({ id: workers[1].messages[0].id, result: {} });
  await retry;
  client.dispose();
});

test('load and engine errors allow retry with a fresh worker', async () => {
  for (const failure of ['load', 'engine']) {
    const { client, workers } = setup();
    const first = client.request('state', {});
    if (failure === 'load') workers[0].fail();
    else workers[0].reply({ id: workers[0].messages[0].id, error: 'Invalid game request' });
    await assert.rejects(first, failure === 'load' ? /load/ : /Invalid/);
    assert.equal(workers[0].terminated, true);
    const retry = client.request('state', {});
    workers[1].reply({ id: workers[1].messages[0].id, result: {} });
    await retry;
    client.dispose();
  }
});

test('a newer request cancels a running request', async () => {
  const { client, workers } = setup();
  const first = client.request('ai', {});
  const rejected = assert.rejects(first, { name: 'AbortError' });
  const second = client.request('state', {});
  await rejected;
  assert.equal(workers[0].terminated, true);
  workers[1].reply({ id: workers[1].messages[0].id, result: {} });
  await second;
  client.dispose();
});
