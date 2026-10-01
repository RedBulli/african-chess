import init, { game_request } from './pkg/african_chess.js';

let ready;
self.onmessage = async ({ data: { id, action, payload } }) => {
  try {
    await (ready ??= init());
    const result = JSON.parse(game_request(action, JSON.stringify(payload)));
    self.postMessage({ id, result });
  } catch (error) {
    self.postMessage({ id, error: typeof error === 'string' ? error : error.message || 'The game engine could not start. Please try again.' });
  }
};
