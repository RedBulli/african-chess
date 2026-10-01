//! Interpretable value model. Features are always from the side-to-move view.
use crate::{Color, Kind, Position};

pub const FEATURE_NAMES: [&str; 30] = [
    "mg_pawn",
    "mg_giraffe",
    "mg_bishop",
    "mg_elephant",
    "mg_queen",
    "eg_pawn",
    "eg_giraffe",
    "eg_bishop",
    "eg_elephant",
    "eg_queen",
    "mg_frozen_pawn",
    "mg_frozen_giraffe",
    "mg_frozen_bishop",
    "mg_frozen_elephant",
    "mg_frozen_queen",
    "eg_frozen_pawn",
    "eg_frozen_giraffe",
    "eg_frozen_bishop",
    "eg_frozen_elephant",
    "eg_frozen_queen",
    "center_pawn",
    "center_giraffe",
    "center_bishop",
    "center_elephant",
    "center_queen",
    "pawn_advance",
    "mg_king_center",
    "eg_king_center",
    "frozen_king",
    "tempo",
];

fn index(kind: Kind) -> usize {
    match kind {
        Kind::Pawn => 0,
        Kind::Giraffe => 1,
        Kind::Bishop => 2,
        Kind::Elephant => 3,
        Kind::Queen => 4,
        Kind::King => 5,
    }
}
pub fn features(p: &Position) -> [f32; 30] {
    let nonpawns = (0..64)
        .filter_map(|s| p.piece(s))
        .filter(|piece| !matches!(piece.kind, Kind::Pawn | Kind::King))
        .count();
    let mg = (nonpawns as f32 / 14.0).min(1.0);
    let eg = 1.0 - mg;
    let frozen = p.frozen_mask();
    let mut out = [0.0; 30];
    for s in 0..64 {
        let Some(piece) = p.piece(s) else { continue };
        let sign = if piece.color == p.side { 1.0 } else { -1.0 };
        let file = s % 8;
        let rank = s / 8;
        let center = f32::from(file.min(7 - file) + rank.min(7 - rank)) / 6.0;
        let is_frozen = frozen & (1u64 << s) != 0;
        let k = index(piece.kind);
        if k < 5 {
            out[k] += sign * mg;
            out[5 + k] += sign * eg;
            if is_frozen {
                out[10 + k] += sign * mg;
                out[15 + k] += sign * eg;
            }
            out[20 + k] += sign * center;
            if k == 0 {
                let advance = if piece.color == Color::White {
                    rank
                } else {
                    7 - rank
                };
                out[25] += sign * f32::from(advance.saturating_sub(1)) / 5.0;
            }
        } else {
            out[26] += sign * mg * center;
            out[27] += sign * eg * center;
            if is_frozen {
                out[28] += sign;
            }
        }
    }
    out[29] = 1.0;
    out
}

#[derive(Clone, Debug)]
pub struct Model {
    pub weights: [f32; 30],
}
impl Default for Model {
    fn default() -> Self {
        Self {
            weights: [
                1.0, 3.6, 4.2, 4.5, 9.5, 1.0, 3.6, 4.2, 4.5, 9.5, -0.5, -1.8, -1.05, -2.25, -4.75,
                -0.5, -1.8, -1.05, -2.25, -4.75, 0.15, 0.72, 0.42, 0.60, 0.18, 0.40, -0.72, 0.72,
                -0.80, 0.0,
            ],
        }
    }
}
impl Model {
    pub fn evaluate(&self, p: &Position) -> i32 {
        let score: f32 = features(p)
            .iter()
            .zip(self.weights)
            .map(|(f, w)| f * w)
            .sum();
        // Keep heuristic scores disjoint from the search's mate band.
        (score * 100.0).round().clamp(-80_000.0, 80_000.0) as i32
    }
    pub fn encode(&self) -> String {
        format!(
            "AFRICAN_CHESS_VALUE_V1\n{}\n",
            self.weights
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
    pub fn decode(text: &str) -> Result<Self, String> {
        let mut fields = text.split_whitespace();
        if fields.next() != Some("AFRICAN_CHESS_VALUE_V1") {
            return Err("Unsupported value model format".into());
        }
        let mut weights = [0.0; 30];
        for w in &mut weights {
            *w = fields
                .next()
                .ok_or("Model needs 30 coefficients")?
                .parse::<f32>()
                .map_err(|_| "Invalid model coefficient")?;
            if !w.is_finite() || w.abs() > 50.0 {
                return Err("Model coefficients must be finite and within ±50".into());
            }
        }
        if fields.next().is_some() {
            return Err("Unexpected extra model coefficients".into());
        }
        if (weights[0] - 1.0).abs() > 1e-6 || (weights[5] - 1.0).abs() > 1e-6 {
            return Err("Pawn material must be anchored at 1".into());
        }
        Ok(Self { weights })
    }
}
