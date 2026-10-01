use crate::types::offset;
use crate::{Color, Kind, Move, Piece, Position};

const DIAGONALS: [(i8, i8); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const STRAIGHTS: [(i8, i8); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const KNIGHT: [(i8, i8); 8] = [
    (-2, -1),
    (-2, 1),
    (-1, -2),
    (-1, 2),
    (1, -2),
    (1, 2),
    (2, -1),
    (2, 1),
];

impl Position {
    pub fn legal_moves(&self) -> Vec<Move> {
        self.pseudo_moves()
            .into_iter()
            .filter(|m| !self.apply_unchecked(*m).in_check(self.side))
            .collect()
    }
    pub fn frozen(&self, s: u8) -> bool {
        s < 64 && self.frozen_mask() & (1u64 << s) != 0
    }
    pub fn frozen_mask(&self) -> u64 {
        let mut mask = 0;
        for (s, piece) in self.board.iter().enumerate() {
            let Some(Piece {
                color,
                kind: Kind::Bishop,
            }) = piece
            else {
                continue;
            };
            for dr in -1..=1 {
                for df in -1..=1 {
                    if let Some(t) = offset(s as u8, df, dr)
                        && self.board[t as usize].is_some_and(|p| p.color != *color)
                    {
                        mask |= 1u64 << t;
                    }
                }
            }
        }
        mask
    }
    pub fn in_check(&self, color: Color) -> bool {
        let king = self
            .board
            .iter()
            .position(|p| {
                *p == Some(Piece {
                    color,
                    kind: Kind::King,
                })
            })
            .expect("position has both kings");
        self.attacked(king as u8, color.other(), self.frozen_mask())
    }
    fn attacked(&self, target: u8, by: Color, frozen: u64) -> bool {
        for (from, piece) in self.board.iter().enumerate() {
            let Some(piece) = piece else { continue };
            if piece.color != by || frozen & (1u64 << from) != 0 || from == target as usize {
                continue;
            }
            let from = from as u8;
            let df = target as i8 % 8 - from as i8 % 8;
            let dr = target as i8 / 8 - from as i8 / 8;
            let diagonal = df.abs() == dr.abs();
            let straight = df == 0 || dr == 0;
            let hit = match piece.kind {
                Kind::Pawn => df.abs() == 1 && dr == by.forward(),
                Kind::King => df.abs() <= 1 && dr.abs() <= 1,
                Kind::Elephant => df.abs() <= 2 && dr.abs() <= 2,
                Kind::Giraffe => {
                    (df.abs() == 1 && dr.abs() == 2)
                        || (df.abs() == 2 && dr.abs() == 1)
                        || (df == 0 && dr * by.forward() > 0 && self.ray_clear(from, target))
                }
                Kind::Bishop => diagonal && self.ray_clear(from, target),
                Kind::Queen => (diagonal || straight) && self.ray_clear(from, target),
            };
            if hit {
                return true;
            }
        }
        false
    }
    fn ray_clear(&self, from: u8, target: u8) -> bool {
        let df = (target as i8 % 8 - from as i8 % 8).signum();
        let dr = (target as i8 / 8 - from as i8 / 8).signum();
        let mut at = offset(from, df, dr);
        while let Some(s) = at {
            if s == target {
                return true;
            }
            if self.board[s as usize].is_some() {
                return false;
            }
            at = offset(s, df, dr);
        }
        false
    }
    fn add_destination(&self, moves: &mut Vec<Move>, from: u8, to: u8, stationary: bool) {
        if self.board[to as usize].is_some_and(|p| p.color == self.side || p.kind == Kind::King) {
            return;
        }
        if self.board[from as usize].is_some_and(|p| p.kind == Kind::Pawn)
            && (to / 8 == 0 || to / 8 == 7)
        {
            for kind in [Kind::Queen, Kind::Bishop, Kind::Giraffe, Kind::Elephant] {
                moves.push(Move {
                    from,
                    to,
                    promotion: Some(kind),
                    stationary,
                });
            }
        } else {
            moves.push(Move {
                from,
                to,
                promotion: None,
                stationary,
            });
        }
    }
    fn slide(&self, moves: &mut Vec<Move>, from: u8, directions: &[(i8, i8)]) {
        for &(df, dr) in directions {
            let mut at = offset(from, df, dr);
            while let Some(to) = at {
                self.add_destination(moves, from, to, false);
                if self.board[to as usize].is_some() {
                    break;
                }
                at = offset(to, df, dr);
            }
        }
    }
    fn pseudo_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        let frozen = self.frozen_mask();
        for from in 0..64u8 {
            let Some(piece) = self.board[from as usize] else {
                continue;
            };
            if piece.color != self.side || frozen & (1u64 << from) != 0 {
                continue;
            }
            match piece.kind {
                Kind::Pawn => {
                    let step = self.side.forward();
                    if let Some(to) = offset(from, 0, step)
                        && self.board[to as usize].is_none()
                    {
                        self.add_destination(&mut moves, from, to, false);
                        if from / 8 == if self.side == Color::White { 1 } else { 6 }
                            && let Some(two) = offset(to, 0, step)
                            && self.board[two as usize].is_none()
                        {
                            self.add_destination(&mut moves, from, two, false);
                        }
                    }
                    for df in [-1, 1] {
                        if let Some(to) = offset(from, df, step)
                            && self.board[to as usize].is_some_and(|p| p.color != self.side)
                        {
                            self.add_destination(&mut moves, from, to, false);
                        }
                    }
                }
                Kind::Giraffe => {
                    for (df, dr) in KNIGHT {
                        if let Some(to) = offset(from, df, dr) {
                            self.add_destination(&mut moves, from, to, false);
                        }
                    }
                    let mut at = offset(from, 0, self.side.forward());
                    while let Some(to) = at {
                        if self.board[to as usize].is_some() {
                            self.add_destination(&mut moves, from, to, true);
                            break;
                        }
                        at = offset(to, 0, self.side.forward());
                    }
                }
                Kind::Bishop => self.slide(&mut moves, from, &DIAGONALS),
                Kind::Queen => {
                    self.slide(&mut moves, from, &DIAGONALS);
                    self.slide(&mut moves, from, &STRAIGHTS);
                }
                Kind::Elephant | Kind::King => {
                    let radius = if piece.kind == Kind::King { 1 } else { 2 };
                    for dr in -radius..=radius {
                        for df in -radius..=radius {
                            if (df != 0 || dr != 0)
                                && let Some(to) = offset(from, df, dr)
                            {
                                self.add_destination(&mut moves, from, to, false);
                            }
                        }
                    }
                    if piece.kind == Kind::King {
                        self.castles(&mut moves, from, frozen);
                    }
                }
            }
        }
        moves
    }
    fn castles(&self, moves: &mut Vec<Move>, from: u8, frozen: u64) {
        let (home, king_bit, queen_bit) = if self.side == Color::White {
            (4, 1, 2)
        } else {
            (60, 4, 8)
        };
        if from != home || self.in_check(self.side) {
            return;
        }
        for (bit, rook, transit, to, empty) in [
            (
                king_bit,
                home + 3,
                home + 1,
                home + 2,
                vec![home + 1, home + 2],
            ),
            (
                queen_bit,
                home - 4,
                home - 1,
                home - 2,
                vec![home - 1, home - 2, home - 3],
            ),
        ] {
            if self.castling & bit == 0
                || frozen & (1u64 << rook) != 0
                || self.board[rook as usize]
                    != Some(Piece {
                        color: self.side,
                        kind: Kind::Elephant,
                    })
                || empty.iter().any(|&s| self.board[s as usize].is_some())
            {
                continue;
            }
            let step = Move {
                from,
                to: transit,
                promotion: None,
                stationary: false,
            };
            if !self.apply_unchecked(step).in_check(self.side) {
                moves.push(Move {
                    from,
                    to,
                    promotion: None,
                    stationary: false,
                });
            }
        }
    }
}
