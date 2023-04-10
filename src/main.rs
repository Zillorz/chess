#![allow(unused)]
#![windows_subsystem = "windows"]

mod uci;
mod chess;

use std::collections::HashMap;
use std::time::Duration;
use macroquad::audio::{load_sound, play_sound_once, Sound};
use macroquad::{color, hash};
use crate::uci::{Limits, ThreadedUci};

use macroquad::prelude::*;
use macroquad::ui::{root_ui, Skin};
use crate::chess::{Piece, Game, IsSomeAnd, MoveResult, Promotion, PROMOTIONS};

const TL_GRAY: Color = Color::new(0.20, 0.20, 0.20, 0.2);
const TD_GRAY: Color = Color::new(0.10, 0.10, 0.10, 0.4);
const TD_RED: Color = Color::new(0.92, 0.20, 0.20, 0.5);

#[macroquad::main("Chess")]
async fn main() {
    request_new_screen_size(480.0, 360.0);
    next_frame().await;

    let button_style = root_ui().style_builder()
        .font_size(40)
        .color(BEIGE)
        .color_hovered(BROWN)
        .build();

    let checkbox_style = root_ui().style_builder()
        .font_size(40)
        .color(RED)
        .color_selected(GREEN)
        .font_size(32)
        .build();

    let default = root_ui().default_skin();
    root_ui().push_skin(&Skin {
        button_style,
        checkbox_style,
        margin: 5.0,
        ..default
    });

    let mut two_player= false;
    let mut white = true;
    let mut flip = false;

    loop {
        clear_background(GRAY);

        if root_ui().button(None, "Play") {
           play_game(two_player, if white { chess::Color::White } else { chess::Color::Black}, !flip && !white).await;
        }

        root_ui().checkbox(hash!(), "Two player?", &mut two_player);
        root_ui().checkbox(hash!(), "Are you playing with white?", &mut white);
        root_ui().checkbox(hash!(), "Is white always on the bottom?", &mut flip);
        next_frame().await;
    }
}

async fn play_game(two_player: bool, player_color: chess::Color, flipped: bool) {
    let wp = load_texture("assets/wP.png").await.unwrap();
    let wn = load_texture("assets/wN.png").await.unwrap();
    let wb = load_texture("assets/wB.png").await.unwrap();
    let wr = load_texture("assets/wR.png").await.unwrap();
    let wq = load_texture("assets/wQ.png").await.unwrap();
    let wk = load_texture("assets/wK.png").await.unwrap();

    let bp = load_texture("assets/bP.png").await.unwrap();
    let bn = load_texture("assets/bN.png").await.unwrap();
    let bb = load_texture("assets/bB.png").await.unwrap();
    let br = load_texture("assets/bR.png").await.unwrap();
    let bq = load_texture("assets/bQ.png").await.unwrap();
    let bk = load_texture("assets/bK.png").await.unwrap();

    let default = load_sound("assets/default.ogg").await.unwrap();
    let castle = load_sound("assets/castle.ogg").await.unwrap();
    let capture = load_sound("assets/capture.ogg").await.unwrap();

    let check_sound = load_sound("assets/check.ogg").await.unwrap();

    let sounds = [default, capture, castle];

    let square_1 = load_texture("assets/square_1.png").await.unwrap();
    let square_2 = load_texture("assets/square_2.png").await.unwrap();

    let get_texture = |piece: Piece| -> Texture2D {
        match piece {
            Piece::WPawn => { wp }
            Piece::WKnight => { wn }
            Piece::WBishop => { wb }
            Piece::WRook => { wr }
            Piece::WQueen => { wq }
            Piece::WKing => { wk }
            Piece::BPawn => { bp }
            Piece::BKnight => { bn }
            Piece::BBishop => { bb }
            Piece::BRook => { br }
            Piece::BQueen => { bq }
            Piece::BKing => { bk }
        }
    };

    let mut game = Game::default();

    // let two_player = true;
    // let player_color = chess::Color::Black;
    // let flipped = false;

    let screen_size = 1024.0;
    let square_size = screen_size / 8.0;
    request_new_screen_size(screen_size, screen_size);
    next_frame().await;

    let mut selected_piece = None;

    let sf = ThreadedUci::new_delay(Duration::from_millis(1_000));
    let limits = Limits::default().time(1_500);

    if game.turn == !player_color && !two_player {
        sf.recommend_move(game, limits);
    }

    let mut winner = None;
    let mut draw = false;

    let mut animations: Vec<Animation> = Vec::new();

    // convert y and x
    let yc = |y: usize| if !flipped { 7 - y } else { y };
    let xc = |x: usize| if flipped { 7 - x } else { x };

    let rp = |u: usize| (xc(u % 8) as f32 * square_size, yc(u / 8) as f32 * square_size);
    let bp = |s: usize| (xc(s % 8), yc(s / 8));

    let mut promotion_square: Option<usize> = None;

    let handle_move = |a1: Option<Animation>, a2: Option<Animation>, mut sound: Sound, res: MoveResult,
                       game: &Game, animations: &mut Vec<Animation>, winner: &mut Option<chess::Color>, draw: &mut bool| {
        if !res.is_ok() { return; }

        if res == MoveResult::Checkmate { *winner = Some(!game.turn); }
        else if res == MoveResult::Check {
            let pos = game.find_king(game.turn).unwrap();

            let px = xc(pos % 8);
            let py = yc(pos / 8);

            let ca = check_animation(game.turn, ((px as f32 + 0.5) * square_size, (py as f32 + 0.5) * square_size), square_size / 2.0);
            animations.push(ca);

            sound = check_sound;
        } else if res == MoveResult::Stalemate || res == MoveResult::Draw {
            *draw = true;
        }

        if let Some(a) = a1 { animations.push(a); }
        if let Some(a) = a2 { animations.push(a); }
        play_sound_once(sound);
    };

    loop {
        clear_background(WHITE);

        if game.turn == !player_color && !two_player {
            if let Some((s_pos, e_pos, pr, alg)) = sf.try_result() {
                let a1 = primary_animation(&game, s_pos, e_pos, rp, bp);
                let a2 = secondary_animation(&game, s_pos, e_pos, rp, bp);
                let mut sound = get_sound(&game, s_pos, e_pos, sounds);

                let res = game.move_checked(s_pos, e_pos, pr);
                assert!(res.is_ok(), "Move {} was illegal at fen={}", alg, game.as_fen());

                handle_move(a1, a2, sound, res, &game, &mut animations, &mut winner, &mut draw);
            }
        }

        for iy in 0..8 {
            let y = square_size * iy as f32;
            let mut x = 0.0;

            for ix in 0..8 {
                if (iy + ix) % 2 == 0 {
                    draw_texture(square_2, x, y, WHITE);
                } else {
                    draw_texture(square_1, x, y, WHITE);
                }

                x += square_size;
            }
        }

        if let Some(winner) = winner {
            let pos = game.find_king(!winner).unwrap();

            let px = xc(pos % 8);
            let py = yc(pos / 8);

            draw_circle((px as f32 + 0.5) * square_size, (py as f32 + 0.5) * square_size, square_size / 2.0, TD_RED);
        } else if draw {
            let pos = game.find_king(chess::Color::White).unwrap();

            let px = xc(pos % 8);
            let py = yc(pos / 8);

            draw_circle((px as f32 + 0.5) * square_size, (py as f32 + 0.5) * square_size, square_size / 2.0, TD_GRAY);

            let pos = game.find_king(chess::Color::Black).unwrap();

            let px = xc(pos % 8);
            let py = yc(pos / 8);

            draw_circle((px as f32 + 0.5) * square_size, (py as f32 + 0.5) * square_size, square_size / 2.0, TD_GRAY);
        }

        // play all animations
        let mut i = 0;
        while animations.len() > i {
            let animation = &mut animations[i];

            if animation.draw_frame(get_texture) {
                i += 1;
            } else {
                animations.remove(i);
            }
        }

        for x in 0..8 {
            'outer: for y in 0..8 {
                let piece = game.board[yc(y) * 8 + xc(x)];

                let dx = (square_size) * x as f32;
                let dy = (square_size) * y as f32;

                for animation in &animations {
                    if let Some(r) = animation.render_exception() {
                        if r.0 == x && r.1 == y { continue 'outer; }
                    }
                }

                if let Some(piece) = piece {
                    draw_texture(get_texture(piece), dx, dy, WHITE);
                }
            }
        }

        if let Some(pos) = promotion_square {
            let color = game.board[pos].unwrap().color();

            let mut promotions: HashMap<usize, Piece> = HashMap::new();

            if (color == chess::Color::White && !flipped) || (color == chess::Color::Black && flipped) {
                let (dx, mut dy) = rp(pos);

                draw_rectangle(dx, dy, square_size, square_size * 4.0, WHITE);

                dy += square_size * 3.0;
                let mut of = 32;
                for i in PROMOTIONS {
                    let piece = Piece::from_promotion(i, color);
                    draw_texture(get_texture(piece),
                                 dx, dy, WHITE);

                    of -= 8;
                    promotions.insert(pos - of, piece);

                    dy -= square_size;
                }
            } else {
                // render down to up
                let (dx, mut dy) = rp(pos);
                dy -= square_size * 3.0;
                draw_rectangle(dx, dy, square_size, square_size * 4.0, WHITE);

                let mut of = 32;
                for i in PROMOTIONS {
                    let piece = Piece::from_promotion(i, color);
                    draw_texture(get_texture(piece),
                                 dx, dy, WHITE);

                    of -= 8;
                    promotions.insert(pos + of, piece);

                    dy += square_size;
                }
            }

            if is_mouse_button_pressed(MouseButton::Left) {
                let (x1, y1) = mouse_position();

                let px = (x1 / square_size).floor() as usize;
                let py = (y1 / square_size).floor() as usize;

                let c_pos = yc(py) * 8 + xc(px);

                if let Some(promotion) = promotions.remove(&c_pos) {
                    game.board[pos] = Some(promotion);
                    promotion_square = None;
                }

                if game.is_in_checkmate(game.turn) { winner = Some(!game.turn); }
                else if game.is_in_check(game.turn) {
                    let pos = game.find_king(game.turn).unwrap();

                    let px = xc(pos % 8);
                    let py = yc(pos / 8);

                    let ca = check_animation(game.turn, ((px as f32 + 0.5) * square_size, (py as f32 + 0.5) * square_size), square_size / 2.0);
                    animations.push(ca);

                    play_sound_once(check_sound);
                } else if game.is_draw() || game.is_stalemate() {
                    draw = true;
                }
            }

            next_frame().await;
            continue;
        }

        // handle moving a piece
        if is_mouse_button_pressed(MouseButton::Left) && selected_piece.is_some() && !draw && winner.is_none() {
            if let Some((x, y)) = selected_piece {
                let (x1, y1) = mouse_position();

                let px = (x1 / square_size).floor() as usize;
                let py = (y1 / square_size).floor() as usize;

                let s_pos = yc(y) * 8 + xc(x);
                let e_pos = yc(py) * 8 + xc(px);

                let a1 = primary_animation(&game, s_pos, e_pos, rp, bp);
                let a2 = secondary_animation(&game, s_pos, e_pos, rp, bp);
                let mut sound = get_sound(&game, s_pos, e_pos, sounds);

                let res = game.move_checked(s_pos, e_pos, None);
                if res.is_ok() {
                    if !two_player { sf.recommend_move(game, limits); }

                    handle_move(a1, a2, sound, res, &game, &mut animations, &mut winner, &mut draw);
                    selected_piece = None;
                } else if res == MoveResult::MissingPromotion && game.is_legal_move(s_pos, e_pos, Some(Promotion::Queen)).is_ok() {
                    let o_pawn = game.board[s_pos];
                    game.move_checked(s_pos, e_pos, Some(Promotion::Queen));
                    game.board[e_pos] = o_pawn;

                    promotion_square = Some(e_pos);
                    selected_piece = None;
                } else {
                    let px = (x1 / square_size).floor() as usize;
                    let py = (y1 / square_size).floor() as usize;

                    let pos = yc(py) * 8 + xc(px);

                    if game.board[pos].some_and(|x| x.color() == game.turn) {
                        selected_piece = Some((px, py));
                    } else { selected_piece = None; }
                }
            }
        }
        else if is_mouse_button_pressed(MouseButton::Left) && (game.turn == player_color || two_player) {
            let (x, y) = mouse_position();

            let px = (x / square_size).floor() as usize;
            let py = (y / square_size).floor() as usize;

            let pos = yc(py) * 8 + xc(px);

            if game.board[pos].some_and(|x| x.color() == game.turn) {
                selected_piece = Some((px, py));
            }
        }

        if let Some((x, y)) = selected_piece {
            // render circle on piece, render possible moves in little circles
            let g_pos = yc(y) * 8 + xc(x);

            draw_circle((x as f32 + 0.5) * square_size, (y as f32 + 0.5) * square_size, square_size / 2.0 - square_size / 5.0, TL_GRAY);

            for pos in game.all_legal_moves(g_pos) {
                let y = yc(pos / 8);
                let x = xc(pos % 8);

                if game.board[pos].is_some() || (game.en_passant.some_and(|x| x.location() == pos)
                    && game.board[g_pos].some_and(|x| *x == Piece::BPawn || *x == Piece::WPawn)) {
                    draw_circle((x as f32 + 0.5) * square_size, (y as f32 + 0.5) * square_size, square_size / 10.0, TD_RED);
                } else {
                    draw_circle((x as f32 + 0.5) * square_size, (y as f32 + 0.5) * square_size, square_size / 10.0, TD_GRAY);
                }
            }
        }

        next_frame().await;
    }
}

#[derive(Debug)]
enum AnimationType {
    // end_pos, no_render_pos
    Move(f32, f32, usize, usize),
    // radius
    Check(f32),
    Disappear,
}

#[derive(Debug)]
struct Animation {
    animation_type: AnimationType,
    piece: Piece,
    position: (f32, f32),
    remaining_time: f32,
    total_time: f32
}

impl Animation {
    fn draw_frame(&mut self, texture_provider: impl FnOnce(Piece) -> Texture2D) -> bool {
        self.remaining_time -= get_frame_time();

        if 0.0 >= self.remaining_time {
            // animation is over
            return false;
        }

        let progress = (self.total_time - self.remaining_time) / self.total_time;

        match self.animation_type {
            AnimationType::Move(ex, ey, _, _) => {
                draw_texture(texture_provider(self.piece),
                             (ex - self.position.0) * progress + self.position.0,
                             (ey - self.position.1) * progress + self.position.1,
                             WHITE);
            }
            AnimationType::Disappear => {
                draw_texture(texture_provider(self.piece), self.position.0, self.position.1,
                             Color::new(1.0, 1.0, 1.0, 1.0 - progress))
            }
            AnimationType::Check(r) => {
                let opacity = (0.5 - (progress - 0.5).abs()) * 2.0;
                let mut color = TD_RED;
                color.a = opacity;

                draw_circle(self.position.0, self.position.1, r, color);
            }
        }

        true
    }

    fn render_exception(&self) -> Option<(usize, usize)> {
        match self.animation_type {
            AnimationType::Move(_, _, ux, uy) => { Some((ux, uy)) }
            _ => { None }
        }
    }
}

const ANIMATION_TIME: f32 = 0.1;
fn primary_animation(game: &Game, from: usize, to: usize,
                                render_location: impl FnOnce(usize) -> (f32, f32) + Copy,
                                block_location: impl FnOnce(usize) -> (usize, usize)) -> Option<Animation> {
    let Some(piece) = game.board[from] else { return None; };

    let (ex, ey) = render_location(to);
    let (ux, uy) = block_location(to);

    Some(Animation {
        animation_type: AnimationType::Move(ex, ey, ux, uy),
        piece,
        position: render_location(from),
        remaining_time: ANIMATION_TIME,
        total_time: ANIMATION_TIME,
    })
}

fn secondary_animation(game: &Game, from: usize, to: usize,
                                  render_location: impl FnOnce(usize) -> (f32, f32) + Copy,
                                  block_location: impl FnOnce(usize) -> (usize, usize)) -> Option<Animation> {
    let Some(piece) = game.board[from] else { return None; };

    // check if move is en_passant
    if let Some(en_passant) = game.en_passant {
        if en_passant.location() == to && (piece == Piece::BPawn || piece == Piece::WPawn) {
            let Some(lost) = game.board[en_passant.pawn_lost_pos()] else { return None; };

            return Some(Animation {
                animation_type: AnimationType::Disappear,
                piece: lost,
                position: render_location(en_passant.pawn_lost_pos()),
                remaining_time: ANIMATION_TIME,
                total_time: ANIMATION_TIME,
            })
        }
    }

    if (piece == Piece::BKing || piece == Piece::WKing) && (to % 8).abs_diff(from % 8) == 2 {
        let (rook_from, rook_to) = if to % 8 > from % 8 {
            (from + 3, to - 1)
        } else {
            (from - 4, to + 1)
        };

        let (ex, ey) = render_location(rook_to);
        let (ux, uy) = block_location(rook_to);

        let Some(rook) = game.board[rook_from] else { return None; };

        return Some(Animation {
            animation_type: AnimationType::Move(ex, ey, ux, uy),
            piece: rook,
            position: render_location(rook_from),
            remaining_time: ANIMATION_TIME,
            total_time: ANIMATION_TIME,
        })
    }
    
    if let Some(taken) = game.board[to] {
        return Some(Animation {
            animation_type: AnimationType::Disappear,
            piece: taken,
            position: render_location(to),
            remaining_time: ANIMATION_TIME,
            total_time: ANIMATION_TIME,
        })
    }
    
    None
}

fn check_animation(color: chess::Color, center: (f32, f32), radius: f32) -> Animation {
    Animation {
        animation_type: AnimationType::Check(radius),
        piece: match color {
            chess::Color::White => { Piece::WKing }
            chess::Color::Black => { Piece::BKing }
        },
        position: center,
        remaining_time: ANIMATION_TIME * 5.0,
        total_time: ANIMATION_TIME * 5.0,
    }
}

fn get_sound(game: &Game, from: usize, to: usize, sounds: [Sound; 3]) -> Sound {
    let Some(piece) = game.board[from] else { return sounds[0]; };

    // check if move is en_passant
    if let Some(en_passant) = game.en_passant {
        if en_passant.location() == to && (piece == Piece::BPawn || piece == Piece::WPawn) {
            return sounds[1];
        }
    }

    if (piece == Piece::BKing || piece == Piece::WKing) && (to % 8).abs_diff(from % 8) == 2 {
        return sounds[2];
    }

    if let Some(taken) = game.board[to] {
        return sounds[1];
    }

    sounds[0]
}