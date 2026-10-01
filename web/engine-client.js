// Keep synchronous Rust search off the UI thread. Only one request is active.
export class EngineClient {
  constructor({
    createWorker = () => new Worker(new URL('./engine-worker.js', import.meta.url), { type: 'module' }),
    timeout = 20_000,
  } = {}) {
    this.createWorker = createWorker;
    this.timeout = timeout;
    this.worker = null;
    this.pending = null;
    this.sequence = 0;
  }

  request(action, payload, signal) {
    if (signal?.aborted) return Promise.reject(new DOMException('Request cancelled', 'AbortError'));
    this.pending?.cancel();
    return new Promise((resolve, reject) => {
      let worker;
      try { worker = this.worker ??= this.createWorker(); }
      catch { reject(new Error('Could not load the game engine. Please try again.')); return; }
      const id = ++this.sequence;
      const finish = (error, result) => {
        clearTimeout(timer);
        signal?.removeEventListener('abort', cancel);
        worker.removeEventListener('message', receive);
        worker.removeEventListener('error', crashed);
        worker.removeEventListener('messageerror', crashed);
        this.pending = null;
        if (error) {
          worker.terminate();
          this.worker = null;
          reject(error);
        } else resolve(result);
      };
      const cancel = () => finish(new DOMException('Request cancelled', 'AbortError'));
      const receive = ({ data }) => {
        if (data?.id !== id) return;
        finish(data.error ? new Error(data.error) : null, data.result);
      };
      const crashed = event => {
        event.preventDefault();
        finish(new Error('Could not load or run the game engine. Please try again.'));
      };
      const timer = setTimeout(() => finish(new Error(
        'The engine took too long. Try again or choose a lower strength.',
      )), this.timeout);
      this.pending = { cancel };
      worker.addEventListener('message', receive);
      worker.addEventListener('error', crashed);
      worker.addEventListener('messageerror', crashed);
      signal?.addEventListener('abort', cancel, { once: true });
      try { worker.postMessage({ id, action, payload }); }
      catch (error) { finish(error); }
    });
  }

  dispose() {
    this.pending?.cancel();
    this.worker?.terminate();
    this.worker = null;
  }
}
