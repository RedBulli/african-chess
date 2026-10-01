use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}
impl Color {
    pub fn other(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
    pub fn forward(self) -> i8 {
        if self == Self::White { 1 } else { -1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Pawn,
    Giraffe,
    Bishop,
    Elephant,
    Queen,
    King,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    pub color: Color,
    pub kind: Kind,
}
impl Piece {
    pub fn fen(self) -> char {
        let c = match self.kind {
            Kind::Pawn => 'p',
            Kind::Giraffe => 'n',
            Kind::Bishop => 'b',
            Kind::Elephant => 'r',
            Kind::Queen => 'q',
            Kind::King => 'k',
        };
        if self.color == Color::White {
            c.to_ascii_uppercase()
        } else {
            c
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub promotion: Option<Kind>,
    pub stationary: bool,
}
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", square_name(self.from), square_name(self.to))?;
        if self.stationary {
            write!(f, "@")?;
        }
        if let Some(k) = self.promotion {
            write!(
                f,
                "{}",
                Piece {
                    color: Color::Black,
                    kind: k
                }
                .fen()
            )?;
        }
        Ok(())
    }
}
pub fn square(s: &str) -> Result<u8, String> {
    let b = s.as_bytes();
    if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) {
        return Err(format!("Invalid square: {s}"));
    }
    Ok((b[1] - b'1') * 8 + b[0] - b'a')
}
pub fn square_name(s: u8) -> String {
    format!("{}{}", (b'a' + s % 8) as char, (b'1' + s / 8) as char)
}
pub(crate) fn offset(s: u8, df: i8, dr: i8) -> Option<u8> {
    let f = s as i8 % 8 + df;
    let r = s as i8 / 8 + dr;
    if (0..8).contains(&f) && (0..8).contains(&r) {
        Some((r * 8 + f) as u8)
    } else {
        None
    }
}
