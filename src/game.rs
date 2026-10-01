use crate::{Color, Kind, Move, Piece, Position};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Checkmate(Color),
    Stalemate,
    Repetition,
    FiftyMove,
    BareKings,
}
impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ongoing => "ongoing",
            Self::Checkmate(_) => "checkmate",
            Self::Stalemate => "stalemate",
            Self::Repetition => "threefold_claim",
            Self::FiftyMove => "fifty_move_claim",
            Self::BareKings => "bare_kings",
        }
    }
    pub fn value(self, side: Color) -> Option<f32> {
        match self {
            Self::Ongoing => None,
            Self::Checkmate(winner) => Some(if winner == side { 1.0 } else { -1.0 }),
            _ => Some(0.0),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct PositionKey {
    board: [Option<Piece>; 64],
    side: Color,
    castling: u8,
}
impl PositionKey {
    fn new(p: &Position) -> Self {
        Self {
            board: p.board,
            side: p.side,
            castling: p.castling,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Game {
    position: Position,
    history: Vec<PositionKey>,
}
impl Game {
    /// The baseline agents always claim threefold/fifty-move draws when present.
    /// A FEN alone has no preceding repetition history.
    pub fn new(position: Position) -> Self {
        let history = vec![PositionKey::new(&position)];
        Self { position, history }
    }
    pub fn position(&self) -> &Position {
        &self.position
    }
    pub fn repetition_count(&self) -> usize {
        let current = self.history.last().expect("game includes current position");
        self.history.iter().filter(|key| *key == current).count()
    }
    pub fn play(&mut self, m: Move) -> Result<(), String> {
        let moves = self.position.legal_moves();
        if self.outcome_with_moves(&moves) != Outcome::Ongoing {
            return Err("Game has already ended".into());
        }
        if !moves.contains(&m) {
            return Err("Illegal move".into());
        }
        *self = self.after_unchecked(m);
        Ok(())
    }
    pub fn play_text(&mut self, s: &str) -> Result<(), String> {
        let m = self
            .position
            .legal_moves()
            .into_iter()
            .find(|m| m.to_string() == s)
            .ok_or_else(|| format!("Illegal move: {s}"))?;
        self.play(m)
    }
    pub fn outcome(&self) -> Outcome {
        self.outcome_with_moves(&self.position.legal_moves())
    }
    pub(crate) fn after_unchecked(&self, m: Move) -> Self {
        let position = self.position.apply_unchecked(m);
        let mut history = if position.halfmove == 0 {
            vec![]
        } else {
            self.history.clone()
        };
        history.push(PositionKey::new(&position));
        Self { position, history }
    }
    pub(crate) fn outcome_with_moves(&self, moves: &[Move]) -> Outcome {
        if moves.is_empty() {
            return if self.position.in_check(self.position.side) {
                Outcome::Checkmate(self.position.side.other())
            } else {
                Outcome::Stalemate
            };
        }
        if self
            .position
            .board
            .iter()
            .flatten()
            .all(|p| p.kind == Kind::King)
        {
            return Outcome::BareKings;
        }
        if self.position.halfmove >= 100 {
            return Outcome::FiftyMove;
        }
        let current = self
            .history
            .last()
            .expect("game history includes current position");
        if self.history.iter().filter(|key| *key == current).count() >= 3 {
            return Outcome::Repetition;
        }
        Outcome::Ongoing
    }
}
