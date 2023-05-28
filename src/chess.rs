use std::collections::HashSet;
use std::ops::{Index, IndexMut, Not};
use bitflags::bitflags;

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
    BKing
}

impl Piece {
    // assuming in bounds
    // Note: does not check pawn movement, as pawn movement is far too complex
    // Note: does not check castling, as castling is also far too complex
    fn can_move(&self, relative_x: isize, relative_y: isize) -> bool {
        match self {
            Piece::WPawn | Piece::BPawn => { true }
            Piece::WKnight | Piece::BKnight => { (relative_x.abs() == 2 && relative_y.abs() == 1) || (relative_y.abs() == 2 && relative_x.abs() == 1) }
            Piece::WBishop | Piece::BBishop => { relative_x.abs() == relative_y.abs() }
            Piece::WRook | Piece::BRook => { (relative_x == 0 && relative_y != 0) || (relative_x != 0 && relative_y == 0) }
            Piece::WKing | Piece::BKing => { relative_x.abs() <= 1 && relative_y.abs() <= 1 }
            Piece::WQueen | Piece::BQueen => {
                (relative_x == 0 && relative_y != 0) || (relative_x != 0 && relative_y == 0) || relative_x.abs() == relative_y.abs()
            }
        }
    }

    pub(crate) fn color(&self) -> Color {
        match self {
            Piece::WPawn | Piece::WKnight | Piece::WBishop | Piece::WRook | Piece::WQueen | Piece::WKing => {
                Color::White
            }
            Piece::BPawn | Piece::BKnight | Piece::BBishop | Piece::BRook | Piece::BQueen | Piece::BKing => {
                Color::Black
            }
        }
    }

    pub(crate) fn from_promotion(prm: Promotion, color: Color) -> Piece {
        match (prm, color) {
            (Promotion::Knight, Color::White) => { Piece::WKnight }
            (Promotion::Bishop, Color::White) => { Piece::WBishop }
            (Promotion::Rook, Color::White) => { Piece::WRook }
            (Promotion::Queen, Color::White) => { Piece::WQueen }
            (Promotion::Knight, Color::Black) => { Piece::BKnight }
            (Promotion::Bishop, Color::Black) => { Piece::BBishop }
            (Promotion::Rook, Color::Black) => { Piece::BRook }
            (Promotion::Queen, Color::Black) => { Piece::BQueen }
        }
    }

    fn from_letter(letter: char) -> Option<Piece> {
        let piece = match letter {
            'p' => { Piece::BPawn }
            'n' => { Piece::BKnight }
            'b' => { Piece::BBishop }
            'r' => { Piece::BRook }
            'q' => { Piece::BQueen }
            'k' => { Piece::BKing }

            'P' => { Piece::WPawn }
            'N' => { Piece::WKnight }
            'B' => { Piece::WBishop }
            'R' => { Piece::WRook }
            'Q' => { Piece::WQueen }
            'K' => { Piece::WKing }
            _ => { return None; }
        };

        Some(piece)
    }

    fn to_letter(self) -> char {
        match self {
            Piece::BPawn => { 'p' }
            Piece::BKnight => { 'n' }
            Piece::BBishop => { 'b' }
            Piece::BRook => { 'r' }
            Piece::BQueen => { 'q' }
            Piece::BKing => { 'k' }

            Piece::WPawn => { 'P' }
            Piece::WKnight => { 'N' }
            Piece::WBishop => { 'B' }
            Piece::WRook => { 'R' }
            Piece::WQueen => { 'Q' }
            Piece::WKing => { 'K' }
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Board([Option<Piece>; 64]);

impl Default for Board {
    fn default() -> Self {
        // keep in mind, this is upside down
        // or just use the fen
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
    fn from_fen_board(fen_board: &str) -> Option<Board> {
        let rows = fen_board.split('/').rev().flat_map(|x| x.chars());

        let mut vec = Vec::new();
        for char in rows {
            if char.is_ascii_digit() {
                for _ in 0..char as u8 - b'0' { vec.push(None); }
            } else {
                vec.push(Piece::from_letter(char));
            }
        }

        let b: [Option<Piece>; 64] = vec.try_into().ok()?;
        Some(Board(b))
    }

    fn into_fen_board(self) -> String {
        let mut str = String::new();

        for y in (0..8).rev() {
            let ym = y * 8;
            let mut none_inr = 0;

            for x in 0..8 {
                if let Some(piece) = self[ym + x] {
                    if none_inr != 0 { str.push(char::from(none_inr as u8 + b'0')); }
                    str.push(piece.to_letter());

                    none_inr = 0;
                } else {
                    none_inr += 1;
                }
            }

            if none_inr != 0 { str.push(char::from(none_inr as u8 + b'0')); }
            if y != 0 { str.push('/') }
        }

        str
    }
}

impl Index<usize> for Board {
    type Output = Option<Piece>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Board {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum EnPassant {
    A2, B2, C2, D2, E2, F2, G2, H2,
    A5, B5, C5, D5, E5, F5, G5, H5
}

impl EnPassant {
    pub(crate) fn location(self) -> usize {
        match self {
            EnPassant::A2 => { 16 }
            EnPassant::B2 => { 17 }
            EnPassant::C2 => { 18 }
            EnPassant::D2 => { 19 }
            EnPassant::E2 => { 20 }
            EnPassant::F2 => { 21 }
            EnPassant::G2 => { 22 }
            EnPassant::H2 => { 23 }
            EnPassant::A5 => { 40 }
            EnPassant::B5 => { 41 }
            EnPassant::C5 => { 42 }
            EnPassant::D5 => { 43 }
            EnPassant::E5 => { 44 }
            EnPassant::F5 => { 45 }
            EnPassant::G5 => { 46 }
            EnPassant::H5 => { 47 }
        }
    }

    // From the location the pawn moves from
    fn from_pawn_location(location: usize) -> Option<EnPassant> {
        let ret = match location {
            8 => { EnPassant::A2 }
            9 => { EnPassant::B2 }
            10 => { EnPassant::C2 }
            11 => { EnPassant::D2 }
            12 => { EnPassant::E2 }
            13 => { EnPassant::F2 }
            14 => { EnPassant::G2 }
            15 => { EnPassant::H2 }
            48 => { EnPassant::A5 }
            49 => { EnPassant::B5 }
            50 => { EnPassant::C5 }
            51 => { EnPassant::D5 }
            52 => { EnPassant::E5 }
            53 => { EnPassant::F5 }
            54 => { EnPassant::G5 }
            55 => { EnPassant::H5 }
            _ => { return None; }
        };

        Some(ret)
    }

    fn from_take_location(location: usize) -> Option<EnPassant> {
        let ret = match location {
            16 => { EnPassant::A2 }
            17 => { EnPassant::B2 }
            18 => { EnPassant::C2 }
            19 => { EnPassant::D2 }
            20 => { EnPassant::E2 }
            21 => { EnPassant::F2 }
            22 => { EnPassant::G2 }
            23 => { EnPassant::H2 }
            40 => { EnPassant::A5 }
            41 => { EnPassant::B5 }
            42 => { EnPassant::C5 }
            43 => { EnPassant::D5 }
            44 => { EnPassant::E5 }
            45 => { EnPassant::F5 }
            46 => { EnPassant::G5 }
            47 => { EnPassant::H5 }
            _ => { return None; }
        };

        Some(ret)
    }

    pub(crate) fn pawn_lost_pos(self) -> usize {
        self.location() + 8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Color {
    White, Black
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self::Output {
        match self {
            Color::White => { Color::Black }
            Color::Black => { Color::White }
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
    fm_clock: u16
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

pub(crate) const PROMOTIONS: [Promotion; 4] = [Promotion::Bishop, Promotion::Rook, Promotion::Knight, Promotion::Queen];
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Promotion {
    Knight, Bishop, Rook, Queen
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
            MoveResult::Valid => { true }
            MoveResult::Check => { true }
            MoveResult::Checkmate => { true }
            MoveResult::Stalemate => { true }
            MoveResult::Draw => { true }
            MoveResult::MissingPromotion => { false }
            MoveResult::Illegal => { false }
            MoveResult::Impossible => { false }
        }
    }
}

impl Game {
    // creates fen representation of game
    pub(crate) fn as_fen(&self) -> String {
        let mut fen = self.board.into_fen_board();

        fen.push(' ');
        match self.turn {
            Color::White => { fen.push('w'); }
            Color::Black => { fen.push('b'); }
        }

        fen.push(' ');
        if self.castle & CastleFlags::WK == CastleFlags::WK { fen.push('K') }
        if self.castle & CastleFlags::WQ == CastleFlags::WQ { fen.push('Q') }
        if self.castle & CastleFlags::BK == CastleFlags::BK { fen.push('k') }
        if self.castle & CastleFlags::BQ == CastleFlags::BQ { fen.push('q') }

        if self.castle == CastleFlags::NONE { fen.push('-') }

        fen.push(' ');
        if let Some(en_passant) = self.en_passant {
            let y = char::from((en_passant.location() / 8 + '1' as usize) as u8);
            let x = char::from((en_passant.location() % 8 + 'a' as usize) as u8);

            fen.push(x);
            fen.push(y);
        } else {
            fen.push('-');
        }

        fen.push(' ');
        fen.push_str(&self.hm_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fm_clock.to_string());

        fen
    }

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
                'K' => { cle |= CastleFlags::WK; }
                'Q' => { cle |= CastleFlags::WQ; }
                'k' => { cle |= CastleFlags::BK; }
                'q' => { cle |= CastleFlags::BQ; }
                '-' => { break }
                _ => {}
            }
        }

        let en_p = if en_passant == "-" { None } else {
            let mut iter = en_passant.chars();

            let x = iter.next()? as usize - 'a' as usize;
            let y = (iter.next()? as usize - '1' as usize) * 8;

            EnPassant::from_take_location(y + x)
        };

        Some(Self {
            board: Board::from_fen_board(board)?,
            en_passant: en_p,
            castle: cle,
            turn: if turn == "w" { Color::White } else { Color::Black },
            hm_clock: hm.parse().ok()?,
            fm_clock: fm.parse().ok()?
        })
    }

    pub(crate) fn find_king(&self, player: Color) -> Option<usize> {
        for (p, pi) in self.board.0.iter().copied().enumerate() {
            let Some(piece) = pi else { continue; };

            if piece.color() == player && (piece == Piece::WKing || piece == Piece::BKing) {
                return Some(p);
            }
        }
        None
    }

    pub(crate) fn is_in_check(&self, player: Color) -> bool {
        // check test
        // Use is_legal_checkless to see if player can check another, as you don't actually take in a check,
        // just threaten to do so, so pins don't matter
        // both players can't be in check, so we assume the opponent of the 'player' is not in check

        let mut kpos = self.find_king(player).unwrap();
        let mut game = *self;
        game.turn = !player;

        let mut in_check = false;
        for (pos, piece) in self.board.0.iter().copied().enumerate() {
            let Some(piece) = piece else { continue; };

            if piece.color() != player && piece != Piece::WKing && piece != Piece::BKing {
                // promotion just in case check is from pawn about to promote
                let res = game.is_legal_checkless(pos, kpos, Some(Promotion::Queen), false);

                if res == MoveResult::Valid {
                    in_check = true;
                    break;
                }
            }
        }

        in_check
    }

    pub(crate) fn is_in_checkmate(&self, player: Color) -> bool {
        // checkmate test
        // both players can't be in check, so we assume the opponent of the 'player' is not in check
        // we also assume it is 'player' turn, as it cannot be the opponents turn while player is in check
        // similar to check test!

        let mut kpos = self.find_king(player).unwrap();
        let mut game = *self;
        game.turn = !player;

        let mut threat_squares = HashSet::new();
        let mut block_pos = Vec::new();

        for (pos, piece) in self.board.0.iter().copied().enumerate() {
            let Some(piece) = piece else { continue; };

            if piece.color() != player && piece != Piece::WKing && piece != Piece::BKing {
                // promotion just in case check is from pawn about to promote
                let res = game.is_legal_checkless(pos, kpos, Some(Promotion::Queen), false);

                if res == MoveResult::Valid {
                    match piece {
                        Piece::WPawn | Piece::WKnight | Piece::BPawn | Piece::BKnight => {
                            // only threat square is the one the piece is on
                            threat_squares.insert(pos);
                        }
                        Piece::WBishop | Piece::WRook | Piece::WQueen | Piece::BBishop | Piece::BRook | Piece::BQueen => {
                            let (mut ox, mut oy) = ((pos % 8) as isize, (pos / 8) as isize);
                            let (nx, ny) = ((kpos % 8) as isize, (kpos / 8) as isize);

                            let rx = (nx - ox).signum();
                            let ry = (ny - oy).signum();

                            // no path tracing bounds checks as those were already done in the is_legal_checkless method
                            while ox != nx && oy != ny {
                                threat_squares.insert((oy * 8 + ox) as usize);

                                ox += rx;
                                oy += ry;
                            }
                        }
                        _ => { }
                    }
                }
            }  else if piece.color() == player && piece != Piece::WKing && piece != Piece::BKing {
                block_pos.push(pos);
            }
        }

        let mut escapable = false;
        game.turn = player;

        let legal_move_wcheck = |from: usize, to: usize| -> bool {
            let legal = game.is_legal_checkless(from, to, Some(Promotion::Queen), false) == MoveResult::Valid;

            if legal {
                let mut n_board = game;
                n_board.move_unchecked(from, to, Some(Promotion::Queen));

                // cannot play a move which puts self in check (or a move which keeps self in check)
                return !n_board.is_in_check(game.turn);
            }

            legal
        };

        // try all king moves
        // straight king moves
        escapable |= legal_move_wcheck(kpos, kpos.saturating_add(1));
        escapable |= legal_move_wcheck(kpos, kpos.saturating_sub(1));
        escapable |= legal_move_wcheck(kpos, kpos.saturating_add(8));
        escapable |= legal_move_wcheck(kpos, kpos.saturating_sub(8));

        // diagonal king moves
        escapable |= legal_move_wcheck(kpos, kpos.saturating_sub(9));
        escapable |= legal_move_wcheck(kpos, kpos.saturating_sub(7));
        escapable |= legal_move_wcheck(kpos, kpos.saturating_add(9));
        escapable |= legal_move_wcheck(kpos, kpos.saturating_add(7));

        // try en passant!!
        if let Some(en_passant) = game.en_passant {
            threat_squares.insert(en_passant.location());
        }

        // try blocking all checks
        for spos in block_pos {
            for ts in &threat_squares {
                escapable |= legal_move_wcheck(spos, *ts);

                if escapable {
                    break;
                }
            }
        }

        !escapable
    }

    pub(crate) fn is_draw(&self) -> bool {
        if self.hm_clock == 100 { return true; }

        let pieces: Vec<(usize, Piece)> = self.board.0.iter().enumerate()
            .filter_map(|x| {
                if x.1.is_some() { Some((x.0, x.1.unwrap())) }
                else { None }
            }).collect();


        if pieces.len() == 2 {
            return true;
        } else if pieces.len() == 3 {
            // gets piece which isnt a king
            let (_, nk) = *pieces.iter().find(|x| x.1 != Piece::BKing && x.1 != Piece::WKing).unwrap();

            if nk == Piece::WKnight || nk == Piece::WBishop || nk == Piece::BKnight || nk == Piece::BBishop {
                return true;
            }
        } else if pieces.len() == 4 {
            // gets last 2 pieces which arent kings
            let nk: Vec<(usize, Piece)> = pieces.iter().filter(|x| x.1 != Piece::BKing && x.1 != Piece::WKing).copied().collect();

            if (nk[0].1 == Piece::BBishop || nk[0].1 == Piece::WBishop) && (nk[1].1 == Piece::BBishop || nk[1].1 == Piece::WBishop) &&
                (nk[0].1.color() != nk[1].1.color()) && (nk[0].0 % 2 == nk[1].0 % 2) {

                return true;
            }
        }

        false
    }

    pub(crate) fn is_stalemate(&self) -> bool {
        if self.is_in_check(self.turn) { return false; }

        for (pos, piece) in self.board.0.iter().copied().enumerate() {
            let Some(piece) = piece else { continue; };

            if piece.color() == self.turn {
                // not in stalemate or check, valid move!
                if !self.all_legal_moves(pos).is_empty() { return false; }
            }
        }

        // No legal moves
        true
    }
    
    pub(crate) fn all_legal_moves(&self, loc: usize) -> Vec<usize> {
        let Some(piece) = self.board[loc] else {
            return Vec::new();
        };

        if piece.color() != self.turn { return Vec::new(); }

        let legal_move = |to: usize| -> bool {
            let legal = self.is_legal_checkless(loc, to, Some(Promotion::Queen), false) == MoveResult::Valid;

            if legal {
                let mut n_board = *self;
                n_board.move_unchecked(loc, to, Some(Promotion::Queen));

                // cannot play a move which puts self in check (or a move which keeps self in check)
                return !n_board.is_in_check(self.turn);
            }

            legal
        };

        let mut list = Vec::new();

        let mut test_move = |to: isize| -> bool {
            if to < 0 { return false; }
            if legal_move(to as usize) { list.push(to as usize); return true; }
            false
        };

        let loc = loc as isize;
        match piece {
            // try move twice, move once, take, and en passant (regular taking moves check for en passant!)
            Piece::WPawn  => {
                test_move(loc + 8);
                test_move(loc + 16);
                test_move(loc + 7);
                test_move(loc + 9);
            }
            Piece::BPawn => {
                test_move(loc - 8);
                test_move(loc - 16);
                test_move(loc - 7);
                test_move(loc - 9);
            }
            // try all knight moves
            Piece::WKnight | Piece::BKnight => {
                test_move(loc + 15);
                test_move(loc + 17);

                test_move(loc - 15);
                test_move(loc - 17);

                test_move(loc + 10);
                test_move(loc - 6);

                test_move(loc - 10);
                test_move(loc + 6);
            }
            // try all bishop moves
            Piece::WBishop | Piece::BBishop => {
                let mut rx = 1;
                let mut ry = 1;

                for i in 0..4 {
                    if i == 1 { rx = -1; }
                    if i == 2 { ry = -1; }
                    if i == 3 { rx = 1; }

                    let (mut lx, mut ly) = (loc % 8, loc / 8);
                    lx += rx;
                    ly += ry;

                    while test_move(ly * 8 + lx) {
                        lx += rx;
                        ly += ry;
                    }
                }
            }
            // try all rook moves
            Piece::WRook | Piece::BRook => {
                let mut rx = 1;
                let mut ry = 0;

                for i in 0..4 {
                    if i == 1 { rx = -1; }
                    if i == 2 { rx = 0; ry = 1; }
                    if i == 3 { ry = -1; }

                    let (mut lx, mut ly) = (loc % 8, loc / 8);
                    lx += rx;
                    ly += ry;

                    while test_move(ly * 8 + lx) {
                        lx += rx;
                        ly += ry;
                    }
                }
            }
            // try all rook and bishop moves
            Piece::WQueen | Piece::BQueen => {
                let mut rx = 1;
                let mut ry = 1;

                for i in 0..8 {
                    if i == 1 { rx = -1; }
                    if i == 2 { ry = -1; }
                    if i == 3 { rx = 1; }
                    if i == 4 { ry = 0; }
                    if i == 5 { rx = -1; }
                    if i == 6 { rx = 0; ry = 1; }
                    if i == 7 { ry = -1; }

                    let (mut lx, mut ly) = (loc % 8, loc / 8);
                    lx += rx;
                    ly += ry;

                    while test_move(ly * 8 + lx) {
                        lx += rx;
                        ly += ry;
                    }
                }
            }
            // castle + king moves
            Piece::WKing | Piece::BKing => {
                test_move(loc + 1);
                test_move(loc - 1);
                test_move(loc + 8);
                test_move(loc - 8);

                test_move(loc + 7);
                test_move(loc + 9);
                test_move(loc - 7);
                test_move(loc - 9);

                // castling
                test_move(loc - 2);
                test_move(loc + 2);
            }
        }

        list
    }

    // validates a moves legality (does not factor in checks/pins)
    // NOTE: checkless validation (except castling, which validates no checks in path)
    fn is_legal_checkless(&self, from: usize, to: usize, promotion: Option<Promotion>, king_check: bool) -> MoveResult {
        // move must be in the board
        if from > 63 || to > 63 {
            return MoveResult::Impossible;
        }

        let Some(piece) = self.board[from] else {
            // can't move a piece that isn't there ??
            return MoveResult::Impossible;
        };

        // Must move your own pieces
        if piece.color() != self.turn { return MoveResult::Impossible; }

        let (ox, oy) = ((from % 8) as isize, (from / 8) as isize);
        let (nx, ny) = ((to % 8) as isize, (to / 8) as isize);

        // make sure move does not take own piece (or enemy king (checkmate?))
        if let Some(piece) = self.board[to] {
            if piece.color() == self.turn || (king_check && piece == Piece::BKing) {
                return MoveResult::Illegal;
            }
        }

        // check if movement pattern is valid for piece
        if piece == Piece::BPawn || piece == Piece::WPawn {
            let rx = (nx - ox).abs();
            let ry = (ny - oy).abs();

            let take = rx == 1 && ry == 1 && self.board[to].is_some();
            let en_passant = self.en_passant.map(|x| x.location() == to).unwrap_or(false) && rx == 1 && ry == 1;
            let regular = ry == 1 && rx == 0 && self.board[to].is_none();

            let occupied = self.board[((oy + (ny - oy).signum()) * 8 + ox) as usize].is_some() || self.board[to].is_some();
            let first = ry == 2 && rx == 0 && ((piece == Piece::BPawn && oy == 6)  || (piece == Piece::WPawn && oy == 1)) && !occupied;

            let dir = (ny - oy).is_positive() ^ (piece == Piece::BPawn);

            if !(take || en_passant || regular || first) || !dir {
                return MoveResult::Illegal;
            }
        } else if (piece == Piece::BKing || piece == Piece::WKing) && (nx - ox).abs() == 2 && ny == oy {
            if self.is_in_check(self.turn) { return MoveResult::Illegal; }
            // Determine which side we are castling
            let mut game = *self;
            match (piece, nx - ox) {
                // black king-side
                (Piece::BKing, 2) => {
                    if self.castle & CastleFlags::BK == CastleFlags::NONE { return MoveResult::Illegal; }
                    if self.board[61].is_some() || self.board[62].is_some() { return MoveResult::Illegal; }

                    game.move_unchecked(60, 61, None);
                    if game.is_in_check(self.turn) { return MoveResult::Illegal; }
                }
                // black queen-side
                (Piece::BKing, -2) => {
                    if self.castle & CastleFlags::BQ == CastleFlags::NONE { return MoveResult::Illegal; }
                    if self.board[57].is_some() || self.board[58].is_some() || self.board[59].is_some() { return MoveResult::Illegal; }

                    game.move_unchecked(60, 59, None);
                    if game.is_in_check(self.turn) { return MoveResult::Illegal; }
                }
                // white king-side
                (Piece::WKing, 2) => {
                    if self.castle & CastleFlags::WK == CastleFlags::NONE { return MoveResult::Illegal; }
                    if self.board[5].is_some() || self.board[6].is_some() { return MoveResult::Illegal; }

                    game.move_unchecked(4, 5, None);
                    if game.is_in_check(self.turn) { return MoveResult::Illegal; }
                }
                // white queen-side
                (Piece::WKing, -2) => {
                    if self.castle & CastleFlags::WQ == CastleFlags::NONE { return MoveResult::Illegal; }
                    if self.board[1].is_some() || self.board[2].is_some() || self.board[3].is_some(){ return MoveResult::Illegal; }

                    game.move_unchecked(4, 3, None);
                    if game.is_in_check(self.turn) { return MoveResult::Illegal; }
                }

                _ => { return MoveResult::Illegal; }
            }
            return MoveResult::Valid;
        } else if !piece.can_move(nx - ox, ny - oy) {
            return MoveResult::Illegal;
        }

        // path trace queen, bishop, and rook moves
        // if any piece is in the way, the move is invalid (castles are king moves)
        if piece == Piece::BRook || piece == Piece::WRook || piece == Piece::BBishop || piece == Piece::WBishop || piece == Piece::BQueen || piece == Piece::WQueen  {
            let rx = (nx - ox).signum();
            let ry = (ny - oy).signum();

            let mut ocx = ox + rx;
            let mut ocy = oy + ry;

            while ocx != nx || ocy != ny {
                if !(0..=7).contains(&ocy) || !(0..=7).contains(&ocx) { return MoveResult::Illegal; }
                if self.board[(ocy * 8 + ocx) as usize].is_some() { return MoveResult::Illegal; }

                ocx += rx;
                ocy += ry;
            }
        }

        // if double pawn movement, make sure it is the first pawn move (can't en passant)
        if (ny - oy).abs() == 2 && ((piece == Piece::BPawn && oy != 6) || (piece == Piece::WPawn && oy != 1)) {
            return MoveResult::Illegal;
        }

        // make sure pawn doesn't move to last (0 or 7) rank without promoting (can't en passant)
        if ((piece == Piece::BPawn && ny == 0) || (piece == Piece::WPawn && ny == 7)) && promotion.is_none()  {
            return MoveResult::MissingPromotion;
        }

        MoveResult::Valid
    }

    pub(crate) fn is_legal_move(&self, from: usize, to: usize, promotion: Option<Promotion>) -> MoveResult {
        let res = self.is_legal_checkless(from, to, promotion, true);
        if res != MoveResult::Valid { return res; }

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

        // 2) Stalemate, use move_gen on every piece, generating all legal moves,
        // if no legal moves are possible and not in check, stalemate
        if !n_board.is_in_check(!self.turn) {
            if n_board.is_stalemate() {
                return MoveResult::Stalemate;
            }

            MoveResult::Valid
        } else {
            // 3) Checkmate
            // Check if game is over for opponent
            if n_board.is_in_checkmate(!self.turn) {
                return MoveResult::Checkmate;
            }

            // 4) Check
            // Opponent is in check
            MoveResult::Check
        }
    }

    pub(crate) fn move_checked(&mut self, from: usize, to: usize, promotion: Option<Promotion>) -> MoveResult {
        let res = self.is_legal_move(from, to, promotion);

        if res == MoveResult::Illegal || res == MoveResult::Impossible || res == MoveResult::MissingPromotion { return res; }
        self.move_unchecked(from, to, promotion);

        res
    }

    // WARNING: does not check for legality of move
    // returns false if piece did not exist
    // NOTE: this method updates en passant, castling,
    // clocks, turns, and promotions, also verifies promotions (pawn and last ranks)
    fn move_unchecked(&mut self, from: usize, to: usize, promotion: Option<Promotion>) -> bool {
        let Some(piece) = self.board[from] else { return false; };

        if self.turn == Color::Black { self.fm_clock += 1; }

        // check for en passant? both offering and taking
        if piece == Piece::BPawn || piece == Piece::WPawn {
            if let Some(en_p) = self.en_passant {
                if en_p.location() == to {
                    self.board[en_p.pawn_lost_pos()] = None;
                }
            }

            let offering = (to as isize - from as isize).abs() == 16 && EnPassant::from_pawn_location(from).is_some();
            if offering { self.en_passant = EnPassant::from_pawn_location(from); }
            else { self.en_passant = None; }
            self.hm_clock = 0;
        } else {
            // en passant is only available for one move
            self.en_passant = None;
            self.hm_clock += 1;
        }

        // check for forfeiting castling rights
        if let Some(piece) = self.board[from] {
            match piece {
                Piece::WRook => {
                    if from == 0 { self.castle -= CastleFlags::WQ; }
                    else if from == 7 { self.castle -= CastleFlags::WK; }
                }
                Piece::WKing => { self.castle -= CastleFlags::W; }
                Piece::BRook => {
                    if from == 56 { self.castle -= CastleFlags::BQ; }
                    else if from == 63 { self.castle -= CastleFlags::BK; }
                }
                Piece::BKing => { self.castle -= CastleFlags::B; }
                _ => { }
            }
        }

        // taking a rook also takes castling rights
        if self.board[to].some_and(|x| *x == Piece::BRook || *x == Piece::WRook) {
            if to == 0 { self.castle -= CastleFlags::WQ; }
            else if to == 7 { self.castle -= CastleFlags::WK; }
            else if to == 56 { self.castle -= CastleFlags::BQ; }
            else if to == 63 { self.castle -= CastleFlags::BK; }
        }

        if self.board[to].is_some() { self.hm_clock = 0; }

        #[allow(clippy::unnecessary_unwrap)]
        if (piece == Piece::BPawn || piece == Piece::WPawn) && promotion.is_some() && (to >= 56 || to <= 7) {
            self.board[to] = Some(Piece::from_promotion(promotion.unwrap(), self.turn));
        } else if (piece == Piece::WKing || piece == Piece::BKing) && (to % 8).abs_diff(from % 8) == 2 {
            let (rook_from, rook_to) = if to % 8 > from % 8 {
                (from + 3, to - 1)
            } else {
                (from - 4, to + 1)
            };

            self.board[to] = self.board[from];
            self.board[rook_to] = self.board[rook_from];

            self.board[rook_from] = None;

        } else {
            self.board[to] = self.board[from];
        }

        self.board[from] = None;
        self.turn = !self.turn;

        true
    }
}

pub(crate) trait IsSomeAnd {
    type Item;

    fn some_and(&self, f: impl FnOnce(&Self::Item) -> bool) -> bool;
}

impl<T> IsSomeAnd for Option<T> {
    type Item = T;

    fn some_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        match self {
            None => false,
            Some(x) => f(x),
        }
    }
}
