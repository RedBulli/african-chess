use crate::{Color, Kind, Move, Piece, square};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub(crate) board: [Option<Piece>; 64],
    pub side: Color,
    pub(crate) castling: u8,
    pub halfmove: u32,
    pub fullmove: u32,
}
impl Position {
    pub fn initial() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap()
    }
    pub fn from_fen(s: &str) -> Result<Self, String> {
        let fields: Vec<_> = s.split_whitespace().collect();
        if fields.len() != 6 || !s.is_ascii() {
            return Err("FEN must contain six ASCII fields".into());
        }
        let rows: Vec<_> = fields[0].split('/').collect();
        if rows.len() != 8 {
            return Err("FEN must have eight ranks".into());
        }
        let mut board = [None; 64];
        let mut kings = [0, 0];
        for (row, text) in rows.iter().enumerate() {
            let mut file = 0usize;
            let mut previous_digit = false;
            for c in text.chars() {
                if ('1'..='8').contains(&c) {
                    if previous_digit {
                        return Err("Consecutive empty-square counts in FEN".into());
                    }
                    file += c as usize - '0' as usize;
                    previous_digit = true;
                } else {
                    previous_digit = false;
                    if file >= 8 {
                        return Err("Too many squares on rank".into());
                    }
                    let color = if c.is_ascii_uppercase() {
                        Color::White
                    } else {
                        Color::Black
                    };
                    let kind = match c.to_ascii_lowercase() {
                        'p' => Kind::Pawn,
                        'n' => Kind::Giraffe,
                        'b' => Kind::Bishop,
                        'r' => Kind::Elephant,
                        'q' => Kind::Queen,
                        'k' => Kind::King,
                        _ => return Err(format!("Unknown FEN piece: {c}")),
                    };
                    if kind == Kind::Pawn && (row == 0 || row == 7) {
                        return Err("Unpromoted pawn on back rank".into());
                    }
                    if kind == Kind::King {
                        kings[if color == Color::White { 0 } else { 1 }] += 1;
                    }
                    board[(7 - row) * 8 + file] = Some(Piece { color, kind });
                    file += 1;
                }
                if file > 8 {
                    return Err("Too many squares on rank".into());
                }
            }
            if file != 8 {
                return Err("Each rank must have eight squares".into());
            }
        }
        if kings != [1, 1] {
            return Err("Exactly one king of each color is required".into());
        }
        let side = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err("Side must be w or b".into()),
        };
        let mut castling = 0;
        if fields[2] != "-" {
            for c in fields[2].chars() {
                let (bit, king_sq, rook_sq, color) = match c {
                    'K' => (1, 4, 7, Color::White),
                    'Q' => (2, 4, 0, Color::White),
                    'k' => (4, 60, 63, Color::Black),
                    'q' => (8, 60, 56, Color::Black),
                    _ => return Err("Invalid castling rights".into()),
                };
                if castling & bit != 0 {
                    return Err("Duplicate castling right".into());
                }
                if board[king_sq]
                    != Some(Piece {
                        color,
                        kind: Kind::King,
                    })
                    || board[rook_sq]
                        != Some(Piece {
                            color,
                            kind: Kind::Elephant,
                        })
                {
                    return Err("Castling right requires king and elephant on home squares".into());
                }
                castling |= bit;
            }
        }
        let halfmove = fields[4]
            .parse::<u32>()
            .map_err(|_| "Invalid halfmove counter")?;
        let fullmove = fields[5]
            .parse::<u32>()
            .map_err(|_| "Invalid fullmove counter")?;
        if fullmove == 0 {
            return Err("Fullmove number must be positive".into());
        }
        // Validate legacy FEN metadata, but discard it: this variant has no en passant.
        if fields[3] != "-" {
            let target = square(fields[3])?;
            let (rank, pawn_sq, origin) = if side == Color::White {
                (5, target.checked_sub(8), target.checked_add(8))
            } else {
                (2, target.checked_add(8), target.checked_sub(8))
            };
            if target / 8 != rank
                || board[target as usize].is_some()
                || halfmove != 0
                || pawn_sq.and_then(|s| board.get(s as usize).copied().flatten())
                    != Some(Piece {
                        color: side.other(),
                        kind: Kind::Pawn,
                    })
                || origin
                    .and_then(|s| board.get(s as usize).copied().flatten())
                    .is_some()
            {
                return Err("Invalid en-passant target or pawn".into());
            }
        }
        Ok(Self {
            board,
            side,
            castling,
            halfmove,
            fullmove,
        })
    }
    pub fn to_fen(&self) -> String {
        let mut out = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                if let Some(piece) = self.board[rank * 8 + file] {
                    if empty > 0 {
                        out.push(char::from(b'0' + empty));
                        empty = 0;
                    }
                    out.push(piece.fen());
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                out.push(char::from(b'0' + empty));
            }
            if rank > 0 {
                out.push('/');
            }
        }
        let rights: String = [(1, 'K'), (2, 'Q'), (4, 'k'), (8, 'q')]
            .iter()
            .filter(|(bit, _)| self.castling & bit != 0)
            .map(|(_, c)| *c)
            .collect();
        format!(
            "{out} {} {} - {} {}",
            if self.side == Color::White { "w" } else { "b" },
            if rights.is_empty() { "-" } else { &rights },
            self.halfmove,
            self.fullmove
        )
    }
    pub fn piece(&self, s: u8) -> Option<Piece> {
        self.board.get(s as usize).copied().flatten()
    }
    pub fn play(&self, m: Move) -> Result<Self, String> {
        if !self.legal_moves().contains(&m) {
            return Err("Illegal move".into());
        }
        Ok(self.apply_unchecked(m))
    }
    pub fn play_text(&self, s: &str) -> Result<Self, String> {
        self.legal_moves()
            .into_iter()
            .find(|m| m.to_string() == s)
            .map(|m| self.apply_unchecked(m))
            .ok_or_else(|| format!("Illegal move: {s}"))
    }
    pub(crate) fn apply_unchecked(&self, m: Move) -> Self {
        let mut next = self.clone();
        let piece = self.board[m.from as usize].expect("generated move has a piece");
        let capture = self.board[m.to as usize].is_some();
        if m.stationary {
            next.board[m.to as usize] = None;
        } else {
            next.board[m.from as usize] = None;
            next.board[m.to as usize] = Some(Piece {
                kind: m.promotion.unwrap_or(piece.kind),
                ..piece
            });
            if piece.kind == Kind::King && m.from.abs_diff(m.to) == 2 {
                let (from, to) = if m.to > m.from {
                    (m.from + 3, m.from + 1)
                } else {
                    (m.from - 4, m.from - 1)
                };
                next.board[to as usize] = next.board[from as usize];
                next.board[from as usize] = None;
            }
        }
        if piece.kind == Kind::King {
            next.castling &= if piece.color == Color::White { !3 } else { !12 };
        }
        for (sq, bit) in [(0, 2), (7, 1), (56, 8), (63, 4)] {
            if m.from == sq || m.to == sq {
                next.castling &= !bit;
            }
        }
        next.halfmove = if piece.kind == Kind::Pawn || capture {
            0
        } else {
            self.halfmove.saturating_add(1)
        };
        next.fullmove = self
            .fullmove
            .saturating_add(u32::from(self.side == Color::Black));
        next.side = self.side.other();
        next
    }
    /// Move-tree count; draw adjudication is intentionally excluded, as in perft.
    pub fn perft(&self, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = self.legal_moves();
        if depth == 1 {
            return moves.len() as u64;
        }
        moves
            .into_iter()
            .map(|m| self.apply_unchecked(m).perft(depth - 1))
            .sum()
    }
}
