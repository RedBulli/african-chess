use crate::{Color, Game, Kind, Move, Outcome, Position};
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct Limits {
    pub depth: u8,
    pub nodes: u64,
}
#[derive(Debug, PartialEq)]
pub struct Analysis {
    pub best: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
}
const MATE: i32 = 100_000;
const INF: i32 = 200_000;
fn material(k: Kind) -> i32 {
    match k {
        Kind::Pawn => 100,
        Kind::Giraffe => 360,
        Kind::Bishop => 420,
        Kind::Elephant => 450,
        Kind::Queen => 950,
        Kind::King => 0,
    }
}
fn center(n: u8) -> i32 {
    i32::from(n.min(7 - n))
}
fn evaluate(p: &Position) -> i32 {
    let frozen = p.frozen_mask();
    let total: i32 = (0..64)
        .filter_map(|s| p.piece(s))
        .map(|x| material(x.kind))
        .sum();
    let mut value = 0;
    for s in 0..64 {
        let Some(piece) = p.piece(s) else { continue };
        let file = s % 8;
        let rank = s / 8;
        let advance = if piece.color == Color::White {
            rank
        } else {
            7 - rank
        };
        let activity = center(file) + center(rank);
        let mut score = material(piece.kind)
            + match piece.kind {
                Kind::Pawn => i32::from(advance) * 8 + center(file) * 5,
                Kind::Giraffe => activity * 12,
                Kind::Bishop => activity * 7,
                Kind::Elephant => activity * 10,
                Kind::Queen => activity * 3,
                Kind::King => {
                    if total > 2800 {
                        -activity * 12 - i32::from(advance) * 8
                    } else {
                        activity * 12
                    }
                }
            };
        if frozen & (1u64 << s) != 0 {
            // A frozen bishop keeps its aura, so retains more of its value.
            score -= match piece.kind {
                Kind::King => 80,
                Kind::Bishop => material(piece.kind) / 4,
                _ => material(piece.kind) / 2,
            };
        }
        value += if piece.color == p.side { score } else { -score };
    }
    value
}
fn order(p: &Position, m: Move) -> i32 {
    let attacker = p.piece(m.from).expect("legal move has piece");
    let victim = p.piece(m.to).map_or_else(
        || {
            if attacker.kind == Kind::Pawn && m.from % 8 != m.to % 8 {
                100
            } else {
                0
            }
        },
        |x| material(x.kind),
    );
    victim * 16 - material(attacker.kind) / 10
        + m.promotion.map_or(0, |k| material(k) * 8)
        + if m.stationary { 10 } else { 0 }
}
fn sorted_moves(p: &Position, mut moves: Vec<Move>, preferred: Option<Move>) -> Vec<Move> {
    moves.sort_by_cached_key(|m| {
        (
            -i32::from(Some(*m) == preferred),
            -order(p, *m),
            m.to_string(),
        )
    });
    moves
}
fn terminal(outcome: Outcome, side: Color, ply: u8) -> Option<i32> {
    match outcome {
        Outcome::Ongoing => None,
        Outcome::Checkmate(winner) => Some(if winner == side {
            MATE - i32::from(ply)
        } else {
            -MATE + i32::from(ply)
        }),
        _ => Some(0),
    }
}
struct Search<'a> {
    nodes: u64,
    limit: u64,
    evaluator: &'a dyn Fn(&Position) -> i32,
    deadline: Option<Instant>,
}
impl Search<'_> {
    fn count(&mut self) -> Result<(), ()> {
        if self.nodes >= self.limit || self.deadline.is_some_and(|d| Instant::now() >= d) {
            return Err(());
        }
        self.nodes += 1;
        Ok(())
    }
    fn negamax(
        &mut self,
        g: &Game,
        depth: u8,
        ply: u8,
        mut alpha: i32,
        beta: i32,
    ) -> Result<i32, ()> {
        self.count()?;
        let moves = g.position().legal_moves();
        if let Some(score) = terminal(g.outcome_with_moves(&moves), g.position().side, ply) {
            return Ok(score);
        }
        if depth == 0 {
            return self.quiet_ready(g, moves, 5, ply, alpha, beta);
        }
        let mut best = -INF;
        for mv in sorted_moves(g.position(), moves, None) {
            let next = g.after_unchecked(mv);
            let score = -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha)?;
            best = best.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        Ok(best)
    }
    fn quiet(&mut self, g: &Game, depth: u8, ply: u8, alpha: i32, beta: i32) -> Result<i32, ()> {
        self.count()?;
        let moves = g.position().legal_moves();
        if let Some(score) = terminal(g.outcome_with_moves(&moves), g.position().side, ply) {
            return Ok(score);
        }
        self.quiet_ready(g, moves, depth, ply, alpha, beta)
    }
    fn quiet_ready(
        &mut self,
        g: &Game,
        moves: Vec<Move>,
        depth: u8,
        ply: u8,
        mut alpha: i32,
        beta: i32,
    ) -> Result<i32, ()> {
        let p = g.position();
        let check = p.in_check(p.side);
        let stand = (self.evaluator)(p);
        if depth == 0 {
            return Ok(stand);
        }
        let mut best = if check { -INF } else { stand };
        if !check {
            if stand >= beta {
                return Ok(stand);
            }
            alpha = alpha.max(stand);
        }
        for mv in sorted_moves(p, moves, None) {
            let pawn_capture =
                p.piece(mv.from).is_some_and(|x| x.kind == Kind::Pawn) && mv.from % 8 != mv.to % 8;
            if !check && p.piece(mv.to).is_none() && !pawn_capture && mv.promotion.is_none() {
                continue;
            }
            let next = g.after_unchecked(mv);
            let score = -self.quiet(&next, depth - 1, ply + 1, -beta, -alpha)?;
            best = best.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        Ok(best)
    }
}
pub fn choose(game: &Game, limits: Limits) -> Result<Analysis, String> {
    choose_with(game, limits, &evaluate)
}
pub fn choose_with(
    game: &Game,
    limits: Limits,
    evaluator: &dyn Fn(&Position) -> i32,
) -> Result<Analysis, String> {
    choose_until(game, limits, evaluator, None)
}
pub fn choose_classic_until(
    game: &Game,
    limits: Limits,
    deadline: Option<Instant>,
) -> Result<Analysis, String> {
    choose_until(game, limits, &evaluate, deadline)
}
pub fn choose_until(
    game: &Game,
    limits: Limits,
    evaluator: &dyn Fn(&Position) -> i32,
    deadline: Option<Instant>,
) -> Result<Analysis, String> {
    if !(1..=6).contains(&limits.depth) || limits.nodes == 0 {
        return Err("Search needs depth 1..6 and a positive node budget".into());
    }
    let moves = game.position().legal_moves();
    if let Some(score) = terminal(game.outcome_with_moves(&moves), game.position().side, 0) {
        return Ok(Analysis {
            best: None,
            score,
            depth: 0,
            nodes: 0,
        });
    }
    let mut result = Analysis {
        best: sorted_moves(game.position(), moves.clone(), None)
            .first()
            .copied(),
        score: evaluator(game.position()),
        depth: 0,
        nodes: 0,
    };
    let mut search = Search {
        nodes: 0,
        limit: limits.nodes,
        evaluator,
        deadline,
    };
    'iterations: for depth in 1..=limits.depth {
        let mut best = -INF;
        let mut best_move = result.best;
        let mut alpha = -INF;
        for mv in sorted_moves(game.position(), moves.clone(), result.best) {
            let next = game.after_unchecked(mv);
            let Ok(child) = search.negamax(&next, depth - 1, 1, -INF, -alpha) else {
                break 'iterations;
            };
            let score = -child;
            if score > best {
                best = score;
                best_move = Some(mv)
            }
            alpha = alpha.max(score);
        }
        result.best = best_move;
        result.score = best;
        result.depth = depth;
        if best.abs() > MATE - 100 {
            break;
        }
    }
    result.nodes = search.nodes;
    Ok(result)
}
