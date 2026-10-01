// Game rules and legal moves come exclusively from the Rust engine.
import { EngineClient } from './engine-client.js';
const engine = new EngineClient();
const $ = selector => document.querySelector(selector);
const STORE = 'african-chess:play:v1';
const other = color => color === 'black' ? 'white' : 'black';
const title = text => text[0].toUpperCase() + text.slice(1);
const defaults = () => ({ version: 1, human: 'black', level: 'balanced', moves: [], fen: null, flipped: false });
let model = defaults();
try {
  const saved = JSON.parse(localStorage.getItem(STORE));
  if (saved?.version === 1 && ['black', 'white'].includes(saved.human)
      && ['relaxed', 'balanced', 'challenging'].includes(saved.level)
      && Array.isArray(saved.moves) && saved.moves.length <= 2000
      && saved.moves.every(m => typeof m === 'string' && /^[a-h][1-8][a-h][1-8][qbnr@]?$/.test(m))
      && (saved.fen == null || typeof saved.fen === 'string' && saved.fen.length <= 200)) {
    model = { ...defaults(), ...saved, flipped: Boolean(saved.flipped) };
  }
} catch { /* A fresh game remains available when storage is disabled or invalid. */ }
let state = null, selected = null, busy = false, epoch = 0, controller = null, aiTimer = null;
let error = '', historyLength = -1, canSave = true;

// Original SVG silhouettes; colors are set by CSS for a consistent set.
const drawings = {
  pawn: '<circle class="body" cx="32" cy="17" r="8"/><path class="body" d="M26 25h12l-2 7 4 14H24l4-14zM22 46h20l4 7H18z"/><path class="detail" d="M26 29h12M24 48h16"/>',
  king: '<path class="body" d="M29 8h6v6h6v5h-6v6h-6v-6h-6v-5h6zM22 26q10-7 20 0l-6 13 5 7H23l5-7zM22 46h20l4 7H18z"/><path class="detail" d="M25 32h14M27 41h10M24 49h16"/>',
  queen: '<path class="body" d="M18 21l8 7 6-14 6 14 8-7-6 20H24zM25 41h14l3 5H22zM22 46h20l4 7H18z"/><circle class="body" cx="17" cy="18" r="3"/><circle class="body" cx="32" cy="11" r="3"/><circle class="body" cx="47" cy="18" r="3"/><path class="detail" d="M26 36h12M24 49h16"/>',
  bishop: '<path class="body" d="M32 9q14 11 11 19-2 6-9 7l3 9H27l3-9q-8-1-9-7-3-8 11-19zM24 44h16l6 9H18z"/><circle class="body" cx="32" cy="8" r="2"/><path class="detail" d="M35 16l-8 12M26 38h12M24 49h16"/>',
  giraffe: '<path class="body" d="M22 47l7-27-3-8 7 3 5-3 10 6-1 7-11-2-1 16 8 8zM22 47h21l4 6H18z"/><path class="body" d="M33 15l-1-7M39 14l2-6"/><circle class="spot" cx="41" cy="19" r="1.6"/><path class="detail" d="M28 27l5 2M26 34l6 2M25 41l7 2M24 50h16"/><path class="spot" d="M32 17l-3 2 2 3 3-1z"/>',
  elephant: '<path class="body" d="M14 31q0-14 16-15 16-2 19 13v12q0 6-5 6-4 0-4-4v-6h-3v10h-8v-9h-6v9h-8V32l-4 4-2-3zM18 48h28l2 5H16z"/><path class="detail" d="M34 20q-10 0-9 10 0 10 9 7zM40 29h1M40 34l5 2M22 50h19"/>',
};
function pieceSVG(kind, color = 'black') {
  return `<svg class="piece ${color}" viewBox="0 0 64 64" aria-hidden="true">${drawings[kind]}</svg>`;
}
function save() {
  try { localStorage.setItem(STORE, JSON.stringify(model)); canSave = true; }
  catch { canSave = false; }
  $('#save-note').textContent = canSave ? 'Saved automatically in this browser' : 'Saving unavailable — keep this tab open';
}
function cancel() {
  epoch += 1;
  controller?.abort(); controller = null;
  clearTimeout(aiTimer); aiTimer = null;
  if ($('#promotion').open) $('#promotion').close();
  selected = null; busy = false; error = '';
}
function begin() {
  cancel(); controller = new AbortController(); busy = true; render();
  return { id: epoch, signal: controller.signal };
}
async function request(action, moves, signal) {
  return engine.request(action, { fen: model.fen, moves, human: model.human, level: model.level }, signal);
}
function commit(result) {
  state = result; model.moves = result.history.map(m => m.move);
  selected = null; busy = false; save(); render();
}
function failed(caught, id) {
  if (id !== epoch || caught.name === 'AbortError') return;
  busy = false;
  error = caught.message || 'The game could not be updated. Please try again.';
  render();
}
async function refresh(moves = model.moves) {
  const { id, signal } = begin();
  try {
    const result = await request('state', moves, signal);
    if (id !== epoch) return;
    commit(result); scheduleAI();
  } catch (caught) { failed(caught, id); }
}
function scheduleAI() {
  if (!state || state.outcome !== 'ongoing' || state.turn === model.human) return;
  busy = true; render();
  aiTimer = setTimeout(async () => {
    const { id, signal } = begin();
    try {
      const result = await request('ai', model.moves, signal);
      if (id !== epoch) return;
      commit(result);
    } catch (caught) { failed(caught, id); }
  }, 180);
}
function newGame(human = model.human) {
  cancel();
  model = { ...defaults(), human, level: model.level };
  state = null; historyLength = -1;
  refresh([]);
}
function lastHumanMove() {
  if (!state) return -1;
  return state.history.findLastIndex(move => move.color === model.human);
}
function undo() {
  const index = lastHumanMove();
  if (index >= 0) refresh(model.moves.slice(0, index));
}
function play(move) {
  if (busy || !state || state.turn !== model.human || !state.legal_moves.some(m => m.move === move.move)) return;
  refresh([...model.moves, move.move]);
}
function selectSquare(square) {
  if (busy || !state || state.outcome !== 'ongoing' || state.turn !== model.human) return;
  const destinations = state.legal_moves.filter(m => m.from === selected && m.to === square);
  if (destinations.length > 1 && destinations.every(m => m.promotion)) {
    showPromotion(destinations); return;
  }
  if (destinations.length === 1) { play(destinations[0]); return; }
  const piece = state.pieces.find(p => p.square === square);
  selected = piece?.color === model.human && selected !== square ? square : null;
  render();
}
function showPromotion(moves) {
  const generation = epoch;
  const options = $('#promotion-options'); options.replaceChildren();
  for (const move of moves) {
    const button = document.createElement('button');
    button.setAttribute('aria-label', `Promote to ${move.promotion}`);
    button.innerHTML = `${pieceSVG(move.promotion, model.human)}<span>${title(move.promotion)}</span>`;
    button.addEventListener('click', () => {
      $('#promotion').close();
      if (generation === epoch) play(move);
    });
    options.append(button);
  }
  $('#promotion').showModal();
}
function orientation() { return model.flipped ? other(model.human) : model.human; }
function renderBoard() {
  const board = $('#board');
  const focused = board.contains(document.activeElement) ? document.activeElement.dataset.square : null;
  const whiteBelow = orientation() === 'white';
  const files = whiteBelow ? 'abcdefgh' : 'hgfedcba';
  const ranks = whiteBelow ? '87654321' : '12345678';
  const pieces = new Map((state?.pieces || []).map(p => [p.square, p]));
  const last = state?.history.at(-1);
  const legal = state?.legal_moves.filter(m => m.from === selected) || [];
  const fragment = document.createDocumentFragment();
  for (let row = 0; row < 8; row++) for (let col = 0; col < 8; col++) {
    const square = files[col] + ranks[row], piece = pieces.get(square);
    const targets = legal.filter(m => m.to === square);
    const button = document.createElement('button');
    button.type = 'button'; button.dataset.square = square; button.dataset.piece = piece?.kind || '';
    button.className = 'square';
    if ((files.charCodeAt(col) - 97 + Number(ranks[row]) - 1) % 2 === 0) button.classList.add('dark');
    if (square === last?.from || square === last?.to) button.classList.add('last');
    if (piece?.frozen) button.classList.add('frozen');
    if (selected === square) button.classList.add('selected');
    if (state?.in_check && piece?.kind === 'king' && piece.color === state.turn) button.classList.add('check');
    if (targets.length) button.classList.add('legal');
    if (targets.some(m => m.capture)) button.classList.add('capture-target');
    if (targets.some(m => m.stationary)) button.classList.add('stationary-target');
    let label = `${square}, ${piece ? `${piece.color} ${piece.kind}${piece.frozen ? ', frozen' : ''}` : 'empty'}`;
    if (targets.some(m => m.stationary)) label += ', capture without moving';
    else if (targets.length) label += targets[0].capture ? ', legal capture' : ', legal move';
    button.setAttribute('aria-label', label);
    button.setAttribute('aria-pressed', String(selected === square));
    button.setAttribute('aria-disabled', String(busy || !state || state.turn !== model.human || state.outcome !== 'ongoing'));
    if (piece) button.innerHTML = pieceSVG(piece.kind, piece.color);
    if (piece?.frozen) button.insertAdjacentHTML('beforeend', '<span class="freeze-marker" aria-hidden="true">❄</span>');
    if (targets.some(m => m.stationary)) button.insertAdjacentHTML('beforeend', '<span class="reach-marker" aria-hidden="true">↟</span>');
    if (col === 0) button.insertAdjacentHTML('beforeend', `<span class="coordinate rank-coordinate" aria-hidden="true">${ranks[row]}</span>`);
    if (row === 7) button.insertAdjacentHTML('beforeend', `<span class="coordinate file-coordinate" aria-hidden="true">${files[col]}</span>`);
    button.addEventListener('click', () => selectSquare(square));
    fragment.append(button);
  }
  board.replaceChildren(fragment);
  if (focused) board.querySelector(`[data-square="${focused}"]`)?.focus({ preventScroll: true });
}
function renderPlayers() {
  const bottom = orientation();
  for (const [id, color] of [['top-player', other(bottom)], ['bottom-player', bottom]]) {
    const bar = $(`#${id}`), isHuman = color === model.human;
    bar.querySelector('.avatar').innerHTML = pieceSVG(isHuman ? 'king' : 'giraffe', color);
    bar.querySelector('strong').textContent = isHuman ? 'You' : 'Opponent';
    bar.querySelector('.player-copy span').textContent = `${isHuman ? 'Your side' : title(model.level)} · ${title(color)} pieces`;
    bar.querySelector('.turn-tag').textContent = state?.outcome === 'ongoing' && state.turn === color ? busy && !isHuman ? 'Thinking…' : 'To move' : '';
  }
}
function renderStatus() {
  let heading = 'One moment…', detail = 'Setting up your board.', label = 'LET’S PLAY';
  if (state) {
    label = `${title(state.turn)} to move`;
    if (state.outcome === 'checkmate') {
      label = 'CHECKMATE'; heading = state.winner === model.human ? 'You win.' : 'Well played.';
      detail = `${title(state.winner)} wins by checkmate. Ready for another game?`;
    } else if (state.outcome !== 'ongoing') {
      label = 'GAME OVER'; heading = 'A drawn game.';
      detail = ({ stalemate: 'No legal moves, and no check. The game ends in stalemate.', threefold_claim: 'The same position appeared three times.', fifty_move_claim: 'Fifty moves each without a pawn move or capture.', bare_kings: 'Only the two kings remain.' })[state.outcome] || 'This game has ended.';
    } else if (busy) {
      heading = state.turn !== model.human ? 'Thinking…' : 'One moment…';
      detail = state.turn !== model.human ? 'Your opponent is considering its next move.' : 'Updating the board.';
    } else if (state.turn !== model.human) {
      heading = 'Opponent’s turn.'; detail = 'Use “Try again” if the opponent was interrupted.';
    } else {
      heading = state.in_check ? 'You’re in check.' : 'Your move.';
      detail = state.in_check ? 'Protect your king: move, block, capture, or freeze the attacker.' : 'Select a piece to see its legal moves.';
      const piece = state.pieces.find(p => p.square === selected);
      if (piece?.frozen) detail = `Your ${piece.kind} is frozen. Capture the adjacent enemy bishop to release it.`;
      else if (piece) {
        const moves = state.legal_moves.filter(m => m.from === selected);
        detail = moves.some(m => m.stationary) ? 'Dashed targets are captures without moving. Your giraffe stays on its square.'
          : moves.length ? `${title(piece.kind)} selected. Choose a highlighted square.` : 'This piece has no legal moves in this position.';
      }
    }
  }
  $('#turn-label').textContent = label; $('#status-title').textContent = heading; $('#status-detail').textContent = detail;
  $('#status-dot').className = `status-dot${busy ? ' thinking' : state?.outcome !== 'ongoing' && state ? ' finished' : ''}`;
}
function formatMove(move) { return `${move.from} ${move.capture ? '×' : '→'} ${move.to}${move.stationary ? ' ↟' : ''}${move.promotion ? ' = ' + ({ queen: 'Q', bishop: 'B', giraffe: 'G', elephant: 'E' })[move.promotion] : ''}`; }
function renderHistory() {
  const history = state?.history || [];
  $('#move-count').textContent = String(history.length);
  const list = $('#history');
  if (!history.length) {
    list.innerHTML = '<div class="empty-history"><span aria-hidden="true">↗</span><p>No moves yet.</p><small>Black has the first word.</small></div>';
  } else {
    const rows = [];
    for (const [index, move] of history.entries()) {
      let row = rows.at(-1);
      if (!row || row[move.color] || move.color === 'black') { row = {}; rows.push(row); }
      row[move.color] = { move, index };
    }
    list.replaceChildren();
    rows.forEach((row, index) => {
      const div = document.createElement('div'); div.className = 'history-row';
      const number = document.createElement('span'); number.className = 'move-number'; number.textContent = String(index + 1).padStart(2, '0'); div.append(number);
      for (const color of ['black', 'white']) {
        const cell = document.createElement('span'), entry = row[color];
        cell.textContent = entry ? formatMove(entry.move) : '—';
        if (entry) { cell.title = `${title(color)} ${entry.move.kind}: ${formatMove(entry.move)}${entry.move.stationary ? ' (stays in place)' : ''}`; if (entry.index === history.length - 1) cell.className = 'latest'; }
        div.append(cell);
      }
      list.append(div);
    });
  }
  if (historyLength !== history.length) list.scrollTop = list.scrollHeight;
  historyLength = history.length;
}
function render() {
  document.body.dataset.busy = String(busy); document.body.dataset.turn = state?.turn || '';
  $('#error').hidden = !error; $('#error-message').textContent = error;
  document.querySelectorAll('[data-side]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.side === model.human)));
  $('#strength').value = model.level; $('#strength').disabled = busy;
  $('#strength-hint').textContent = ({ relaxed: 'A gentler opponent while you learn the pieces.', balanced: 'Looks a few moves ahead. Keep an eye on your pieces.', challenging: 'Thinks further ahead and takes a little longer.' })[model.level];
  $('#undo').disabled = lastHumanMove() < 0;
  renderBoard(); renderPlayers(); renderStatus(); renderHistory();
}
$('#new-game').addEventListener('click', () => newGame());
$('#undo').addEventListener('click', undo);
$('#flip').addEventListener('click', () => { model.flipped = !model.flipped; save(); render(); });
$('#retry').addEventListener('click', () => refresh());
$('#strength').addEventListener('change', event => { model.level = event.target.value; save(); render(); });
document.querySelectorAll('[data-side]').forEach(button => button.addEventListener('click', () => { if (button.dataset.side !== model.human) newGame(button.dataset.side); }));
$('#rules-link').addEventListener('click', () => { $('#field-guide').open = true; });
$('#cancel-promotion').addEventListener('click', () => $('#promotion').close());
$('#promotion').addEventListener('close', () => { selected = null; render(); });
$('#board').addEventListener('keydown', event => {
  const buttons = [...$('#board').children], index = buttons.indexOf(event.target);
  const delta = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -8, ArrowDown: 8 }[event.key];
  if (delta && index >= 0) { event.preventDefault(); buttons[Math.max(0, Math.min(63, index + delta))].focus(); }
  else if (event.key === 'Escape') { selected = null; render(); }
});
document.querySelectorAll('.guide-piece').forEach(element => { element.innerHTML = pieceSVG(element.dataset.piece); });
render(); refresh();
