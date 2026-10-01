use crate::learning::Model;
use crate::opponent::{Analysis, Limits, choose_with};
use crate::types::square_name;
use crate::{Color, Game, Kind, Move, Outcome, Piece, Position};

fn color(c: Color) -> &'static str {
    if c == Color::White { "white" } else { "black" }
}
fn kind(k: Kind) -> &'static str {
    match k {
        Kind::Pawn => "pawn",
        Kind::Giraffe => "giraffe",
        Kind::Bishop => "bishop",
        Kind::Elephant => "elephant",
        Kind::Queen => "queen",
        Kind::King => "king",
    }
}
fn promotion(m: Move) -> String {
    m.promotion
        .map(|k| format!("\"{}\"", kind(k)))
        .unwrap_or_else(|| "null".into())
}
fn capture(p: &Position, m: Move) -> bool {
    p.piece(m.to).is_some()
        || (p.piece(m.from).is_some_and(|x| x.kind == Kind::Pawn) && m.from % 8 != m.to % 8)
}
struct Record {
    mv: Move,
    piece: Piece,
    capture: bool,
}
pub struct Session {
    pub game: Game,
    history: Vec<Record>,
    analysis: Option<Analysis>,
}
impl Session {
    pub fn replay(fen: Option<&str>, moves: &[&str]) -> Result<Self, String> {
        if moves.len() > 2000 {
            return Err("Move transcript is too long".into());
        }
        let game = Game::new(match fen {
            Some(f) => Position::from_fen(f)?,
            None => Position::initial(),
        });
        let mut session = Self {
            game,
            history: vec![],
            analysis: None,
        };
        for text in moves {
            let mv = session
                .game
                .position()
                .legal_moves()
                .into_iter()
                .find(|m| m.to_string() == *text)
                .ok_or_else(|| format!("Illegal move: {text}"))?;
            session.push(mv)?;
        }
        Ok(session)
    }
    fn push(&mut self, mv: Move) -> Result<(), String> {
        let p = self.game.position();
        let record = Record {
            mv,
            piece: p.piece(mv.from).expect("legal move has a piece"),
            capture: capture(p, mv),
        };
        self.game.play(mv)?;
        self.history.push(record);
        Ok(())
    }
    pub fn state_json(&self) -> String {
        let p = self.game.position();
        let outcome = self.game.outcome();
        let pieces = (0..64)
            .filter_map(|s| {
                p.piece(s).map(|piece| {
                    format!(
                        "{{\"square\":\"{}\",\"color\":\"{}\",\"kind\":\"{}\",\"frozen\":{}}}",
                        square_name(s),
                        color(piece.color),
                        kind(piece.kind),
                        p.frozen(s)
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        let legal = if outcome == Outcome::Ongoing {
            p.legal_moves()
        } else {
            vec![]
        };
        let legal=legal.iter().map(|&m|format!("{{\"move\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"promotion\":{},\"stationary\":{},\"capture\":{}}}",m,square_name(m.from),square_name(m.to),promotion(m),m.stationary,capture(p,m))).collect::<Vec<_>>().join(",");
        let history=self.history.iter().map(|r|format!("{{\"move\":\"{}\",\"color\":\"{}\",\"kind\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"stationary\":{},\"capture\":{},\"promotion\":{}}}",r.mv,color(r.piece.color),kind(r.piece.kind),square_name(r.mv.from),square_name(r.mv.to),r.mv.stationary,r.capture,promotion(r.mv))).collect::<Vec<_>>().join(",");
        let winner = if let Outcome::Checkmate(winner) = outcome {
            format!("\"{}\"", color(winner))
        } else {
            "null".into()
        };
        let analysis = self
            .analysis
            .as_ref()
            .map(|a| {
                format!(
                    "{{\"depth\":{},\"nodes\":{},\"score\":{}}}",
                    a.depth, a.nodes, a.score
                )
            })
            .unwrap_or_else(|| "null".into());
        format!(
            "{{\"fen\":\"{}\",\"turn\":\"{}\",\"outcome\":\"{}\",\"winner\":{},\"in_check\":{},\"pieces\":[{}],\"legal_moves\":[{}],\"history\":[{}],\"analysis\":{}}}",
            p.to_fen(),
            color(p.side),
            outcome.label(),
            winner,
            p.in_check(p.side),
            pieces,
            legal,
            history,
            analysis
        )
    }
    pub fn ai_turn(&mut self, human: &str, level: &str) -> Result<(), String> {
        let human = match human {
            "black" => Color::Black,
            "white" => Color::White,
            _ => return Err("Human side must be black or white".into()),
        };
        let limits = match level {
            "relaxed" => Limits {
                depth: 2,
                nodes: 8000,
            },
            "balanced" => Limits {
                depth: 3,
                nodes: 50_000,
            },
            "challenging" => Limits {
                depth: 4,
                nodes: 180_000,
            },
            _ => return Err("Unknown strength".into()),
        };
        if self.game.outcome() != Outcome::Ongoing {
            return Err("Game has already ended".into());
        }
        if self.game.position().side == human {
            return Err("It is the human player's turn".into());
        }
        // Bundle the promoted checkpoint so play does not depend on lab run files.
        let model = Model::decode(include_str!("../models/browser.weights"))?;
        let analysis = choose_with(&self.game, limits, &|p| model.evaluate(p))?;
        self.push(analysis.best.ok_or("Opponent has no legal move")?)?;
        self.analysis = Some(analysis);
        Ok(())
    }
}
