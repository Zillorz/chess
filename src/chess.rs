use bitflags::bitflags;
use std::ops::{Index, IndexMut, Not};

bitflags! {
    #[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct CastleFlags: u8 {
        const NONE = 0;

        const WK = 1 << 0;
        const WQ = 1 << 1;
        const BK = 1 << 2;
        const BQ = 1 << 3;

        const W = Self::WK.bits() | Self::WQ.bits();
        const B = Self::BK.bits() | Self::BQ.bits();

        const ALL = Self::W.bits() | Self::B.bits();
    }
}

impl Default for CastleFlags {
    fn default() -> Self {
        CastleFlags::ALL
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Piece {
    WPawn,
    WKnight,
    WBishop,
    WRook,
    WQueen,
    WKing,
    BPawn,
    BKnight,
    BBishop,
    BRook,
    BQueen,
    BKing,
}

impl Piece {
    // assuming in bounds
    // Note: does not check pawn movement, as pawn movement is far too complex
    // Note: does not check castling, as castling is also far too complex
    fn can_move(self, rx: isize, ry: isize) -> bool {
        let (rx, ry) = (rx.abs(), ry.abs()); // pieces can move forward or backward!

        // piece can't do this!
        if rx == 0 && ry == 0 {
            return false;
        }

        match self {
            Piece::WPawn | Piece::BPawn => false, // not handling this
            Piece::WKing | Piece::BKing => rx <= 1 && ry <= 1,
            Piece::WKnight | Piece::BKnight => (rx == 2 && ry == 1) || (rx == 1 && ry == 2),
            Piece::WBishop | Piece::BBishop => rx == ry,
            Piece::WRook | Piece::BRook => (rx > 0 && ry == 0) || (rx == 0 && ry > 0),
            Piece::WQueen | Piece::BQueen => {
                (rx == ry) || (rx > 0 && ry == 0) || (rx == 0 && ry > 0)
            }
        }
    }

    pub(crate) fn color(self) -> Color {
        match self {
            Piece::WPawn
            | Piece::WKnight
            | Piece::WBishop
            | Piece::WRook
            | Piece::WQueen
            | Piece::WKing => Color::White,
            Piece::BPawn
            | Piece::BKnight
            | Piece::BBishop
            | Piece::BRook
            | Piece::BQueen
            | Piece::BKing => Color::Black,
        }
    }

    pub(crate) fn from_promotion(prm: Promotion, color: Color) -> Piece {
        match (prm, color) {
            (Promotion::Knight, Color::White) => Piece::WKnight,
            (Promotion::Bishop, Color::White) => Piece::WBishop,
            (Promotion::Rook, Color::White) => Piece::WRook,
            (Promotion::Queen, Color::White) => Piece::WQueen,
            (Promotion::Knight, Color::Black) => Piece::BKnight,
            (Promotion::Bishop, Color::Black) => Piece::BBishop,
            (Promotion::Rook, Color::Black) => Piece::BRook,
            (Promotion::Queen, Color::Black) => Piece::BQueen,
        }
    }

    fn from_letter(letter: char) -> Option<Piece> {
        let piece = match letter {
            'p' => Piece::BPawn,
            'n' => Piece::BKnight,
            'b' => Piece::BBishop,
            'r' => Piece::BRook,
            'q' => Piece::BQueen,
            'k' => Piece::BKing,

            'P' => Piece::WPawn,
            'N' => Piece::WKnight,
            'B' => Piece::WBishop,
            'R' => Piece::WRook,
            'Q' => Piece::WQueen,
            'K' => Piece::WKing,

            _ => {
                return None;
            }
        };

        Some(piece)
    }

    fn to_letter(self) -> char {
        match self {
            Piece::BPawn => 'p',
            Piece::BKnight => 'n',
            Piece::BBishop => 'b',
            Piece::BRook => 'r',
            Piece::BQueen => 'q',
            Piece::BKing => 'k',

            Piece::WPawn => 'P',
            Piece::WKnight => 'N',
            Piece::WBishop => 'B',
            Piece::WRook => 'R',
            Piece::WQueen => 'Q',
            Piece::WKing => 'K',
        }
    }
}

pub type Pos = (isize, isize);

#[derive(Default)]
pub struct BoardIter {
    state: isize,
}

impl Iterator for BoardIter {
    type Item = (isize, isize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.state >= 64 {
            return None;
        }

        let ret = (self.state % 8, self.state / 8);
        self.state += 1;

        Some(ret)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Board([Option<Piece>; 64]);

impl Default for Board {
    #[rustfmt::skip]
    fn default() -> Self {
        // keep in mind, this is upside down
        // or just use the fen?
        Board([
            Some(Piece::WRook), Some(Piece::WKnight), Some(Piece::WBishop),
            Some(Piece::WQueen), Some(Piece::WKing), Some(Piece::WBishop), Some(Piece::WKnight), Some(Piece::WRook),

            Some(Piece::WPawn), Some(Piece::WPawn), Some(Piece::WPawn), Some(Piece::WPawn),
            Some(Piece::WPawn), Some(Piece::WPawn), Some(Piece::WPawn), Some(Piece::WPawn),

            None, None, None, None, None, None, None, None,
            None, None, None, None, None, None, None, None,
            None, None, None, None, None, None, None, None,
            None, None, None, None, None, None, None, None,

            Some(Piece::BPawn), Some(Piece::BPawn), Some(Piece::BPawn), Some(Piece::BPawn),
            Some(Piece::BPawn), Some(Piece::BPawn), Some(Piece::BPawn), Some(Piece::BPawn),

            Some(Piece::BRook), Some(Piece::BKnight), Some(Piece::BBishop),
            Some(Piece::BQueen), Some(Piece::BKing), Some(Piece::BBishop), Some(Piece::BKnight), Some(Piece::BRook)
        ])
    }
}

impl Board {
    #[allow(unused)]
    fn from_fen_board(fen_board: &str) -> Option<Board> {
        let rows = fen_board.split('/').rev().flat_map(|x| x.chars());

        let mut vec = Vec::new();

        for char in rows {
            if char.is_ascii_digit() {
                for _ in 0..char as u8 - b'0' {
                    vec.push(None);
                }
            } else {
                vec.push(Piece::from_letter(char));
            }
        }

        let b: [Option<Piece>; 64] = vec.try_into().ok()?;
        Some(Board(b))
    }

    #[allow(unused)]
    fn into_fen_board(self) -> String {
        let mut str = String::new();

        for y in (0..8).rev() {
            let mut empty_count = 0;

            for x in 0..8 {
                if let Some(piece) = self[(x, y)] {
                    if empty_count != 0 {
                        str.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    str.push(piece.to_letter());
                } else {
                    empty_count += 1;
                }
            }

            if empty_count != 0 {
                str.push_str(&empty_count.to_string());
                empty_count = 0;
            }

            if y != 0 {
                str.push('/');
            }
        }

        str
    }

    pub(crate) fn find_king(&self, player: Color) -> Option<Pos> {
        for (x, y) in BoardIter::default() {
            let Some(piece) = self[(x, y)] else {
                continue;
            };

            if piece.color() == player && matches!(piece, Piece::WKing | Piece::BKing) {
                return Some((x, y));
            }
        }

        None
    }
}

impl Index<(isize, isize)> for Board {
    type Output = Option<Piece>;

    fn index(&self, (x, y): (isize, isize)) -> &Self::Output {
        &self.0[(x + y * 8).cast_unsigned()]
    }
}

impl IndexMut<(isize, isize)> for Board {
    fn index_mut(&mut self, (x, y): (isize, isize)) -> &mut Self::Output {
        &mut self.0[(x + y * 8).cast_unsigned()]
    }
}

// this represents the square that the pawn which is peforming en passant can move to
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum EnPassant {
    A3,
    B3,
    C3,
    D3,
    E3,
    F3,
    G3,
    H3,
    A6,
    B6,
    C6,
    D6,
    E6,
    F6,
    G6,
    H6,
}

impl EnPassant {
    pub(crate) fn location(self) -> Pos {
        match self {
            EnPassant::A3 => (0, 2),
            EnPassant::B3 => (1, 2),
            EnPassant::C3 => (2, 2),
            EnPassant::D3 => (3, 2),
            EnPassant::E3 => (4, 2),
            EnPassant::F3 => (5, 2),
            EnPassant::G3 => (6, 2),
            EnPassant::H3 => (7, 2),
            EnPassant::A6 => (0, 5),
            EnPassant::B6 => (1, 5),
            EnPassant::C6 => (2, 5),
            EnPassant::D6 => (3, 5),
            EnPassant::E6 => (4, 5),
            EnPassant::F6 => (5, 5),
            EnPassant::G6 => (6, 5),
            EnPassant::H6 => (7, 5),
        }
    }

    // From the location the spawn skips over
    fn from_skipped_location(pos: Pos) -> Option<EnPassant> {
        let ret = match pos {
            (0, 2) => EnPassant::A3,
            (1, 2) => EnPassant::B3,
            (2, 2) => EnPassant::C3,
            (3, 2) => EnPassant::D3,
            (4, 2) => EnPassant::E3,
            (5, 2) => EnPassant::F3,
            (6, 2) => EnPassant::G3,
            (7, 2) => EnPassant::H3,
            (0, 5) => EnPassant::A6,
            (1, 5) => EnPassant::B6,
            (2, 5) => EnPassant::C6,
            (3, 5) => EnPassant::D6,
            (4, 5) => EnPassant::E6,
            (5, 5) => EnPassant::F6,
            (6, 5) => EnPassant::G6,
            (7, 5) => EnPassant::H6,

            _ => return None,
        };

        Some(ret)
    }

    fn from_take_location(location: Pos) -> Option<EnPassant> {
        let ret = match location {
            (0, 3) => EnPassant::A3,
            (1, 3) => EnPassant::B3,
            (2, 3) => EnPassant::C3,
            (3, 3) => EnPassant::D3,
            (4, 3) => EnPassant::E3,
            (5, 3) => EnPassant::F3,
            (6, 3) => EnPassant::G3,
            (7, 3) => EnPassant::H3,
            (0, 4) => EnPassant::A6,
            (1, 4) => EnPassant::B6,
            (2, 4) => EnPassant::C6,
            (3, 4) => EnPassant::D6,
            (4, 4) => EnPassant::E6,
            (5, 4) => EnPassant::F6,
            (6, 4) => EnPassant::G6,
            (7, 4) => EnPassant::H6,
            _ => {
                return None;
            }
        };

        Some(ret)
    }

    pub(crate) fn pawn_lost_pos(self) -> Pos {
        let (x, mut y) = self.location();

        if y == 5 {
            y -= 1;
        } else {
            y += 1;
        }

        (x, y)
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Color {
    White,
    Black,
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self::Output {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Game {
    pub(crate) board: Board,
    // clears after every move
    pub(crate) en_passant: Option<EnPassant>,
    castle: CastleFlags,
    pub(crate) turn: Color,
    // resets on pawn move
    hm_clock: u8,
    fm_clock: u16,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            board: Board::default(),
            en_passant: None,
            castle: CastleFlags::ALL,
            turn: Color::White,
            hm_clock: 0,
            fm_clock: 1,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Promotion {
    Knight,
    Bishop,
    Rook,
    Queen,
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum MoveResult {
    Valid,
    Check,
    Checkmate,
    Stalemate,
    Draw,
    MissingPromotion,
    Illegal,
    Impossible,
}

impl MoveResult {
    pub(crate) fn is_ok(self) -> bool {
        match self {
            MoveResult::Valid
            | MoveResult::Check
            | MoveResult::Checkmate
            | MoveResult::Stalemate
            | MoveResult::Draw => true,
            MoveResult::MissingPromotion | MoveResult::Illegal | MoveResult::Impossible => false,
        }
    }
}

#[derive(PartialEq, Eq)]
enum Legality {
    Legal,
    Illegal,
    IllegalBecauseOfCheck,
}

impl Game {
    // creates fen representation of game
    #[allow(unused)]
    pub(crate) fn as_fen(&self) -> String {
        let mut fen = self.board.into_fen_board();

        fen.push(' ');
        match self.turn {
            Color::White => {
                fen.push('w');
            }
            Color::Black => {
                fen.push('b');
            }
        }

        fen.push(' ');
        if self.castle & CastleFlags::WK == CastleFlags::WK {
            fen.push('K');
        }
        if self.castle & CastleFlags::WQ == CastleFlags::WQ {
            fen.push('Q');
        }
        if self.castle & CastleFlags::BK == CastleFlags::BK {
            fen.push('k');
        }
        if self.castle & CastleFlags::BQ == CastleFlags::BQ {
            fen.push('q');
        }

        if self.castle == CastleFlags::NONE {
            fen.push('-');
        }

        fen.push(' ');
        if let Some(en_passant) = self.en_passant {
            let (x, y) = en_passant.location();

            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            {
                fen.push(char::from(x as u8 + b'a'));
                fen.push(char::from(y as u8 + b'1'));
            }
        } else {
            fen.push('-');
        }

        fen.push(' ');
        fen.push_str(&self.hm_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fm_clock.to_string());

        fen
    }

    #[allow(unused)]
    pub(crate) fn from_fen(fen: impl AsRef<str>) -> Option<Self> {
        let mut parts = fen.as_ref().split(' ');

        let board = parts.next()?;
        let turn = parts.next()?;
        let castle = parts.next()?;
        let en_passant = parts.next()?;
        let hm = parts.next().unwrap_or("0");
        let fm = parts.next().unwrap_or("1");

        let mut cle = CastleFlags::NONE;
        for i in castle.chars() {
            match i {
                'K' => {
                    cle |= CastleFlags::WK;
                }
                'Q' => {
                    cle |= CastleFlags::WQ;
                }
                'k' => {
                    cle |= CastleFlags::BK;
                }
                'q' => {
                    cle |= CastleFlags::BQ;
                }
                '-' => break,
                _ => {}
            }
        }

        let en_p = if en_passant == "-" {
            None
        } else {
            let mut iter = en_passant.chars();

            let x = iter.next()? as isize - 'a' as isize;
            let y = iter.next()? as isize - '1' as isize;

            EnPassant::from_skipped_location((x, y))
        };

        Some(Self {
            board: Board::from_fen_board(board)?,
            en_passant: en_p,
            castle: cle,
            turn: if turn == "w" {
                Color::White
            } else {
                Color::Black
            },
            hm_clock: hm.parse().ok()?,
            fm_clock: fm.parse().ok()?,
        })
    }

    // validates a moves legality (does not factor in checks/pins)
    // NOTE: checkless validation (except castling, which validates no checks in path)
    #[allow(clippy::too_many_lines)] // this function is the vast vast vast majority of the checking
    fn is_legal_checkless(&self, from: Pos, to: Pos, promotion: Option<Promotion>) -> MoveResult {
        // move must be in the board
        let (ox, oy) = from;
        let (nx, ny) = to;

        if ox > 7 || oy > 7 || ox < 0 || oy < 0 || nx > 7 || ny > 7 || nx < 0 || ny < 0 {
            return MoveResult::Impossible;
        }

        let Some(piece) = self.board[(ox, oy)] else {
            // can't move a piece that isn't there ??
            return MoveResult::Impossible;
        };

        // Must move your own pieces
        if piece.color() != self.turn {
            return MoveResult::Impossible;
        }

        // make sure move does not take own piece
        // (or enemy king (checkmate?)) <--- This behavior is actually removed, cuz the game is over
        if let Some(piece) = self.board[(nx, ny)]
            && piece.color() == self.turn
        {
            return MoveResult::Illegal;
        }

        // check if movement pattern is valid for piece
        let rx = nx - ox;
        let ry = ny - oy;

        // pawns are funky
        if matches!(piece, Piece::BPawn | Piece::WPawn) {
            let (arx, ary) = (rx.abs(), ry.abs());

            let occupied = self.board[(nx, ny)].is_some();

            // can only move diagonal if piece is at 'to' pos or en_passant
            let diagonal = arx == 1
                && ary == 1
                && (self.board[(nx, ny)].is_some()
                    || self.en_passant.is_some_and(|x| x.location() == to));

            // when moving 2, nothing can be in between the two locations
            // and pawn must be on inital rank
            let single_trace = self.board[(ox, oy + ry.signum())].is_none();
            let double = ary == 2
                && ((piece == Piece::BPawn && oy == 6) || (piece == Piece::WPawn && oy == 1))
                && single_trace
                && !occupied;

            let basic = arx == 0 && ary == 1 && !occupied;

            // (piece.color() == White) ^ (ry < 0), where ^ is xor
            let direction = (piece.color() == Color::White && ry > 0)
                || (piece.color() == Color::Black && ry < 0);

            if (ny == 0 || ny == 7) && promotion.is_none() {
                return MoveResult::MissingPromotion;
            }

            if !(basic | diagonal | double) || !direction {
                return MoveResult::Illegal;
            }
        }
        // kings can castle, when castling the abs relative x movement of the king is >1
        else if matches!(piece, Piece::BKing | Piece::WKing) && rx.abs() > 1 {
            // cannot castle if we are in check
            if self.is_in_check(self.turn) {
                return MoveResult::Illegal;
            }

            // Determine which side we are castling
            let mut game = *self;

            match (piece, from, to) {
                // black king-side
                (Piece::BKing, (4, 7), (6, 7)) => {
                    if self.castle & CastleFlags::BK == CastleFlags::NONE
                        || self.board[(5, 7)].is_some()
                        || self.board[(6, 7)].is_some()
                    {
                        return MoveResult::Illegal;
                    }

                    game.move_unchecked((4, 7), (5, 7), None);
                    if game.is_in_check(self.turn) {
                        return MoveResult::Illegal;
                    }
                }
                // black queen-side
                (Piece::BKing, (4, 7), (2, 7)) => {
                    if self.castle & CastleFlags::BQ == CastleFlags::NONE
                        || self.board[(3, 7)].is_some()
                        || self.board[(2, 7)].is_some()
                        || self.board[(1, 7)].is_some()
                    {
                        return MoveResult::Illegal;
                    }

                    game.move_unchecked((4, 7), (3, 7), None);
                    if game.is_in_check(self.turn) {
                        return MoveResult::Illegal;
                    }
                }
                // white king-side
                (Piece::WKing, (4, 0), (6, 0)) => {
                    if self.castle & CastleFlags::WK == CastleFlags::NONE
                        || self.board[(5, 0)].is_some()
                        || self.board[(6, 0)].is_some()
                    {
                        return MoveResult::Illegal;
                    }

                    game.move_unchecked((4, 0), (5, 0), None);
                    if game.is_in_check(self.turn) {
                        return MoveResult::Illegal;
                    }
                }
                // white queen-side
                (Piece::WKing, (4, 0), (2, 0)) => {
                    if self.castle & CastleFlags::WQ == CastleFlags::NONE
                        || self.board[(1, 0)].is_some()
                        || self.board[(2, 0)].is_some()
                        || self.board[(3, 0)].is_some()
                    {
                        return MoveResult::Illegal;
                    }

                    game.move_unchecked((4, 0), (3, 0), None);
                    if game.is_in_check(self.turn) {
                        return MoveResult::Illegal;
                    }
                }

                // any other king move beyond arx>1 is illegal
                _ => {
                    return MoveResult::Illegal;
                }
            }
            return MoveResult::Valid;
        } else if !piece.can_move(rx, ry) {
            return MoveResult::Illegal;
        }

        // path trace queen, bishop, and rook moves
        // if any piece is in the way, the move is invalid (castles are king moves)
        if matches!(
            piece,
            Piece::BRook
                | Piece::WRook
                | Piece::BBishop
                | Piece::WBishop
                | Piece::BQueen
                | Piece::WQueen
        ) {
            let sx = rx.signum();
            let sy = ry.signum();

            // traced x, y
            let mut tx = ox + sx;
            let mut ty = oy + sy;

            // continue tracing until we reach location
            while tx != nx || ty != ny {
                // cannot move through piece
                if self.board[(tx, ty)].is_some() {
                    return MoveResult::Illegal;
                }

                tx += sx;
                ty += sy;
            }
        }

        MoveResult::Valid
    }

    pub(crate) fn is_in_check(&self, player: Color) -> bool {
        // Uses is_legal to see if player can check another,
        // as you don't actually take (the king) in a check, just threaten to do so, so pins don't matter.
        // both players can't be in check, so we assume the opponent of the 'player' is not in check

        let kpos = self.board.find_king(player).unwrap();

        let mut game = *self;
        game.turn = !player;

        for pos in BoardIter::default() {
            let Some(piece) = self.board[pos] else {
                continue;
            };

            if piece.color() == player || matches!(piece, Piece::WKing | Piece::BKing) {
                // king is technically in check if other king can take it?
                // this is here because this method is used to stop illegal moves
                if piece.color() != player
                    && matches!(piece, Piece::WKing | Piece::BKing)
                    && Piece::WKing.can_move(kpos.0 - pos.0, kpos.1 - pos.1)
                {
                    return true;
                }

                continue;
            }

            // while this might seem recursive, this is not called on kings, which is the
            // only time is_in_check is called in is_legal_checkless

            if game
                .is_legal_checkless(pos, kpos, Some(Promotion::Queen))
                .is_ok()
            {
                return true;
            }
        }

        false
    }

    // draw, insufficient material and 50 move rule
    // stalemate is NOT INCLUDED
    pub(crate) fn is_draw(&self) -> bool {
        if self.hm_clock == 100 {
            return true;
        }

        // bool -> square color (true -> dark, false -> light)
        let pieces: Vec<(bool, Piece)> = self
            .board
            .0
            .iter()
            .enumerate()
            .filter_map(|(u, p)| {
                if let Some(piece) = p
                    && !matches!(piece, Piece::WKing | Piece::BKing)
                {
                    Some((u % 2 == 0, *piece))
                } else {
                    None
                }
            })
            .collect();

        // draw if
        // - 2 kings
        // - 2 kings + 1 bishop/knight
        // - 2 kings + same colored bishops

        if pieces.is_empty() {
            true
        } else if pieces.len() == 1 {
            matches!(
                pieces[0].1,
                Piece::WKnight | Piece::BKnight | Piece::WBishop | Piece::BBishop
            )
        } else if pieces.len() == 2 {
            pieces[0].1 == Piece::BBishop
                || pieces[0].1 == Piece::WBishop
                    && (pieces[1].1 == Piece::BBishop || pieces[1].1 == Piece::WBishop)
                    && (pieces[0].1.color() != pieces[1].1.color())
                    && pieces[0].0 == pieces[1].0
        } else {
            false
        }
    }

    pub(crate) fn is_in_checkmate(&self, player: Color) -> bool {
        if !self.is_in_check(player) {
            return false;
        }

        for (x, y) in BoardIter::default() {
            let Some(piece) = self.board[(x, y)] else {
                continue;
            };

            if piece.color() == player {
                // just play this move!
                if !self.all_legal_moves((x, y)).is_empty() {
                    return false;
                }
            }
        }

        true
    }

    pub(crate) fn is_stalemate(&self) -> bool {
        if self.is_in_check(self.turn) {
            return false;
        }

        for (x, y) in BoardIter::default() {
            let Some(piece) = self.board[(x, y)] else {
                continue;
            };

            if piece.color() == self.turn {
                // not in stalemate or check, valid move!
                if !self.all_legal_moves((x, y)).is_empty() {
                    return false;
                }
            }
        }

        true
    }

    fn is_legal(&self, from: Pos, to: Pos) -> Legality {
        let legal = self.is_legal_checkless(from, to, Some(Promotion::Queen)) == MoveResult::Valid;

        if legal {
            let mut copy = *self;
            copy.move_unchecked(from, to, Some(Promotion::Queen));

            // cannot play a move which puts self in check (or a move which keeps self in check)
            if copy.is_in_check(self.turn) {
                return Legality::IllegalBecauseOfCheck;
            }
        }

        if legal {
            Legality::Legal
        } else {
            Legality::Illegal
        }
    }

    pub(crate) fn all_legal_moves(&self, start: Pos) -> Vec<Pos> {
        let Some(piece) = self.board[start] else {
            return Vec::new();
        };

        if piece.color() != self.turn {
            return Vec::new();
        }

        let mut list = Vec::new();

        let mut test_move = |rx: isize, ry: isize| -> Legality {
            let (x, y) = (rx + start.0, ry + start.1);

            let legal = self.is_legal(start, (rx, ry));
            if legal == Legality::Legal {
                list.push((x, y));
            }

            legal
        };

        let bishop_directions = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        let rook_directions = [(1, 0), (0, 1), (-1, 0), (0, -1)];

        match piece {
            // try move twice, move once, take, and en passant (regular taking moves check for en passant!)
            Piece::WPawn => {
                test_move(0, 1);
                test_move(0, 2);
                test_move(1, 1);
                test_move(-1, 1);
            }
            Piece::BPawn => {
                test_move(0, -1);
                test_move(0, -2);
                test_move(1, -1);
                test_move(-1, -1);
            }
            // try all knight moves
            Piece::WKnight | Piece::BKnight => {
                test_move(2, 1);
                test_move(1, 2);

                test_move(-2, 1);
                test_move(-1, 2);

                test_move(2, -1);
                test_move(1, -2);

                test_move(-2, -1);
                test_move(-1, -2);
            }
            // try all bishop moves
            Piece::WBishop | Piece::BBishop => {
                for (sx, sy) in bishop_directions {
                    let (mut tx, mut ty) = (sx, sy);

                    while test_move(tx, ty) != Legality::Illegal {
                        tx += sx;
                        ty += sy;
                    }
                }
            }
            // try all rook moves
            Piece::WRook | Piece::BRook => {
                for (sx, sy) in rook_directions {
                    let (mut tx, mut ty) = (sx, sy);

                    while test_move(tx, ty) != Legality::Illegal {
                        tx += sx;
                        ty += sy;
                    }
                }
            }
            // try all rook and bishop moves
            Piece::WQueen | Piece::BQueen => {
                for (sx, sy) in bishop_directions.iter().chain(rook_directions.iter()) {
                    let (mut tx, mut ty) = (*sx, *sy);

                    while test_move(tx, ty) != Legality::Illegal {
                        tx += sx;
                        ty += sy;
                    }
                }
            }
            // castle + king moves
            Piece::WKing | Piece::BKing => {
                test_move(1, 0);
                test_move(0, 1);
                test_move(-1, 0);
                test_move(0, -1);

                test_move(1, 1);
                test_move(-1, -1);
                test_move(1, -1);
                test_move(-1, 1);

                // castling
                test_move(2, 0);
                test_move(-2, 0);
            }
        }

        list
    }

    pub(crate) fn is_legal_move(
        &self,
        from: Pos,
        to: Pos,
        promotion: Option<Promotion>,
    ) -> MoveResult {
        let res = self.is_legal_checkless(from, to, promotion);
        if res != MoveResult::Valid {
            return res;
        }

        // Any move at this point is valid (omitting check)
        let mut n_board = *self;
        n_board.move_unchecked(from, to, promotion);

        // cannot play a move which puts self in check (or a move which keeps self in check)
        if n_board.is_in_check(self.turn) {
            return MoveResult::Illegal;
        }

        // Last 4 move types
        // 1) Draw - Analyze material on n_board,
        // if material is king v king, king & bishop v king, king & knight v king,
        // king and bishop vs king and bishop (same color bishops)
        // or if 50 move rule is done (100 moves on halfmove clock)
        if n_board.is_draw() {
            return MoveResult::Draw;
        }

        if n_board.is_in_check(!self.turn) {
            // 2) Checkmate
            // Check if game is over for opponent
            if n_board.is_in_checkmate(!self.turn) {
                return MoveResult::Checkmate;
            }

            // 3) Check
            // Opponent is in check
            MoveResult::Check
        } else {
            // 4) Stalemate, use move_gen on every piece, generating all legal moves,
            // if no legal moves are possible and not in check, stalemate
            if n_board.is_stalemate() {
                return MoveResult::Stalemate;
            }

            MoveResult::Valid
        }
    }

    pub(crate) fn move_checked(
        &mut self,
        from: Pos,
        to: Pos,
        promotion: Option<Promotion>,
    ) -> MoveResult {
        let res = self.is_legal_move(from, to, promotion);

        if res == MoveResult::Illegal
            || res == MoveResult::Impossible
            || res == MoveResult::MissingPromotion
        {
            return res;
        }
        self.move_unchecked(from, to, promotion);

        res
    }

    // WARNING: does not check for legality of move, this can lead to weird weird results, use
    // move_checked if possible
    // returns false if piece did not exist
    // NOTE: this method updates en passant, castling,
    // clocks, turns, and promotions, also verifies promotions (pawn and last ranks)
    fn move_unchecked(&mut self, from: Pos, to: Pos, promotion: Option<Promotion>) -> bool {
        let Some(piece) = self.board[from] else {
            return false;
        };

        if self.turn == Color::Black {
            self.fm_clock += 1;
        }

        let is_pawn = matches!(piece, Piece::BPawn | Piece::WPawn);

        if is_pawn
            && let Some(en_passant) = self.en_passant
            && en_passant.location() == to
        {
            // take en passant
            self.board[en_passant.pawn_lost_pos()] = None;
        } else if is_pawn && to.1.abs_diff(from.1) == 2 {
            // offer en passant
            self.en_passant = EnPassant::from_take_location(to);
        } else {
            // clear en passant
            self.en_passant = None;
        }

        // increment hm clock
        if !is_pawn && self.board[to].is_none() {
            self.hm_clock += 1;
        } else {
            self.hm_clock = 0;
        }

        if matches!(piece, Piece::WPawn | Piece::BPawn)
            && let Some(promotion) = promotion
            && (to.1 == 0 || to.1 == 7)
        {
            self.board[to] = Some(Piece::from_promotion(promotion, piece.color()));
        } else if matches!(piece, Piece::WKing | Piece::BKing) && to.0.abs_diff(from.0) == 2 {
            // since we don't check move eligiblity, let's just castle with whatever is there

            // if we're moving right, kingside
            if to.0 > from.0 {
                self.board[(to.0 - 1, to.1)] = self.board[(7, to.1)];
                self.board[(7, to.1)] = None;
            }
            // queenside
            else {
                self.board[(to.0 + 1, to.1)] = self.board[(0, to.1)];
                self.board[(0, to.1)] = None;
            }

            self.board[to] = self.board[from];
        } else {
            self.board[to] = self.board[from];
        }

        // piece moved away
        self.board[from] = None;
        // flip the turn
        self.turn = !self.turn;

        // check for forfeiting/taking castling rights
        // just check if the rook and or king is not on the right square
        if self.board[(0, 0)].is_none_or(|x| x != Piece::WRook) {
            self.castle -= CastleFlags::WQ;
        }

        if self.board[(7, 0)].is_none_or(|x| x != Piece::WRook) {
            self.castle -= CastleFlags::WK;
        }

        if self.board[(0, 7)].is_none_or(|x| x != Piece::BRook) {
            self.castle -= CastleFlags::BQ;
        }

        if self.board[(7, 7)].is_none_or(|x| x != Piece::BRook) {
            self.castle -= CastleFlags::BK;
        }

        if self.board[(4, 0)].is_none_or(|x| x != Piece::WKing) {
            self.castle -= CastleFlags::W;
        }

        if self.board[(4, 7)].is_none_or(|x| x != Piece::BKing) {
            self.castle -= CastleFlags::B;
        }

        true
    }

    pub(crate) fn get_move_effects(
        &mut self,
        from: Pos,
        to: Pos,
        promotion: Option<Promotion>,
    ) -> Option<MoveEffects> {
        let piece = self.board[from]?;

        let mut effects = MoveEffects {
            lost_piece: None,
            piece_moves: ((from, to, piece), None),
            gained_piece: None,
        };

        let is_pawn = matches!(piece, Piece::BPawn | Piece::WPawn);

        if is_pawn
            && let Some(en_passant) = self.en_passant
            && en_passant.location() == to
        {
            let loc = en_passant.pawn_lost_pos();
            effects.lost_piece = Some((loc, self.board[loc]?));
        }

        if let Some(p) = self.board[to] {
            effects.lost_piece = Some((to, p));
        }

        if matches!(piece, Piece::WPawn | Piece::BPawn)
            && let Some(promotion) = promotion
            && (to.1 == 0 || to.1 == 7)
        {
            effects.gained_piece =
                Some((to, piece, Piece::from_promotion(promotion, piece.color())));
        } else if matches!(piece, Piece::WKing | Piece::BKing) && to.0.abs_diff(from.0) == 2 {
            // since we don't check move eligiblity, let's just castle with whatever is there

            // if we're moving right, kingside
            if to.0 > from.0 {
                effects.piece_moves.1 = Some(((7, to.1), (to.0 - 1, to.1), self.board[(7, to.1)]?));
            }
            // queenside
            else {
                effects.piece_moves.1 = Some(((0, to.1), (to.0 + 1, to.1), self.board[(0, to.1)]?));
            }
        }

        Some(effects)
    }
}

pub type PieceMove = (Pos, Pos, Piece);

pub(crate) struct MoveEffects {
    // if a piece is taken, including en passant
    pub(crate) lost_piece: Option<(Pos, Piece)>,

    // only 2 if castle
    pub(crate) piece_moves: (PieceMove, Option<PieceMove>),

    // this is for promotion
    // Position of promotion, Piece that promoted, Piece that was promoted to
    pub(crate) gained_piece: Option<(Pos, Piece, Piece)>,
}
