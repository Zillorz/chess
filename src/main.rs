#![allow(unused)]
#![windows_subsystem = "windows"]

mod chess;
mod uci;

use crate::uci::{Limits, ThreadedUci};
use macroquad::audio::{Sound, load_sound, play_sound_once};
use macroquad::{Error, color, hash};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::chess::{
    Board, BoardIter, Game, MoveEffects, MoveResult, PROMOTIONS, Piece, Pos, Promotion,
};
use macroquad::prelude::*;
use macroquad::ui::{Skin, root_ui};

const TL_GRAY: Color = Color::new(0.20, 0.20, 0.20, 0.2);
const TD_GRAY: Color = Color::new(0.10, 0.10, 0.10, 0.4);
const TD_RED: Color = Color::new(0.92, 0.20, 0.20, 0.5);

#[macroquad::main("Chess")]
async fn main() {
    request_new_screen_size(480.0, 360.0);
    next_frame().await;

    let button_style = root_ui()
        .style_builder()
        .font_size(40)
        .color(BEIGE)
        .color_hovered(BROWN)
        .build();

    let checkbox_style = root_ui().style_builder()
        .style_builder()
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

    let mut two_player = false;
    let mut white = true;
    let mut flip = false;

    loop {
        clear_background(GRAY);

        if root_ui().button(None, "Play") {
            play_game(
                two_player,
                if white {
                    chess::Color::White
                } else {
                    chess::Color::Black
                },
                !flip && !white,
            )
            .await;
        }

        root_ui().checkbox(hash!(), "Two player?", &mut two_player);
        root_ui().checkbox(hash!(), "Are you playing with white?", &mut white);
        root_ui().checkbox(hash!(), "Is white always on the bottom?", &mut flip);
        next_frame().await;
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum ChessSound {
    Move,
    Castle,
    Capture,
    Check,
}

struct GuiGame {
    piece_textures: HashMap<Piece, Texture2D>,
    sounds: HashMap<ChessSound, Sound>,
    light_square_texture: Texture2D,
    dark_square_texture: Texture2D,

    animations: Vec<Box<dyn Animation>>,

    top_left: Vec2,
    size: f32,
    flipped: bool,

    complete: bool,

    selected_square: Option<Pos>,
    held: bool,

    promotion: Option<(Pos, Pos, bool)>
}

impl GuiGame {
    async fn load_from_files() -> Result<Self, Error> {
        let piece_textures = HashMap::from([
            (Piece::WPawn, load_texture("assets/wP.png").await?),
            (Piece::WKnight, load_texture("assets/wN.png").await?),
            (Piece::WBishop, load_texture("assets/wB.png").await?),
            (Piece::WRook, load_texture("assets/wR.png").await?),
            (Piece::WQueen, load_texture("assets/wQ.png").await?),
            (Piece::WKing, load_texture("assets/wK.png").await?),
            (Piece::BPawn, load_texture("assets/bP.png").await?),
            (Piece::BKnight, load_texture("assets/bN.png").await?),
            (Piece::BBishop, load_texture("assets/bB.png").await?),
            (Piece::BRook, load_texture("assets/bR.png").await?),
            (Piece::BQueen, load_texture("assets/bQ.png").await?),
            (Piece::BKing, load_texture("assets/bK.png").await?),
        ]);

        let sounds = HashMap::from([
            (ChessSound::Move, load_sound("assets/default.ogg").await?),
            (ChessSound::Castle, load_sound("assets/castle.ogg").await?),
            (ChessSound::Capture, load_sound("assets/capture.ogg").await?),
            (ChessSound::Check, load_sound("assets/check.ogg").await?),
        ]);

        let light_square_texture = load_texture("assets/square_1.png").await?;
        let dark_square_texture = load_texture("assets/square_2.png").await?;

        Ok(GuiGame {
            piece_textures,
            sounds,
            light_square_texture,
            dark_square_texture,

            top_left: Vec2::splat(0.),
            size: 1024.,
            flipped: false,
            complete: false,

            selected_square: None,
            held: false,

            animations: Vec::new(),

            promotion: None
        })
    }

    fn get_texture(&self, piece: Piece) -> &Texture2D {
        self.piece_textures.get(&piece).unwrap()
    }

    fn get_px(&self, x: isize) -> f32 {
        x as f32 * (self.size / 8.) + self.top_left.x
    }

    fn get_py(&self, y: isize) -> f32 {
        (if self.flipped {
            y as f32
        } else {
            (7. - y as f32)
        }) * (self.size / 8.)
            + self.top_left.y
    }

    fn get_x(&self, px: f32) -> isize {
        ((px - self.top_left.x) / (self.size / 8.)) as isize
    }

    fn get_y(&self, py: f32) -> isize {
        (if self.flipped {
            (py - self.top_left.y) / (self.size / 8.)
        } else {
            8. - (py - self.top_left.y) / (self.size / 8.)
        }) as isize
    }
}

async fn play_game(two_player: bool, player_color: chess::Color, flipped: bool) {
    let mut game = Game::default();
    let mut ctx = GuiGame::load_from_files().await.unwrap();

    request_new_screen_size(1024.0, 1024.0);
    next_frame().await;

    // let mut selected_piece = None;

    let sf = ThreadedUci::new_delay(Duration::from_millis(500));
    let limits = Limits::default().depth(18).time(150);

    if game.turn == !player_color && !two_player {
        sf.recommend_move(game, limits);
    }

    // let mut winner = None;
    let mut draw = false;

    // let mut animations: Vec<Animation> = Vec::new();

    loop {
        let size = f32::min(screen_height(), screen_width());
        ctx.size = size;

        render(&game, &ctx);
        ctx.animations
            .retain_mut(|animation| !animation.tick(get_frame_time()));

        if let Some(promotion) = ctx.promotion {
            handle_promotion(&mut game, &mut ctx, promotion);
        } else {
            handle_input(&mut game, &mut ctx);
        }

        next_frame().await;
    }
}

fn render(game: &Game, tctx: &GuiGame) {
    let board = game.board;
    let square_size = tctx.size / 8.0;

    let map_x = |x: isize| tctx.get_px(x);
    let map_y = |y: isize| tctx.get_py(y);

    for (x, y) in BoardIter::default() {
        let is_dark_square = (x + y) % 2 == 0;

        if is_dark_square {
            draw_texture_ex(
                &tctx.dark_square_texture,
                map_x(x),
                map_y(y),
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::splat(square_size)),
                    ..Default::default()
                },
            );
        } else {
            draw_texture_ex(
                &tctx.light_square_texture,
                map_x(x),
                map_y(y),
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::splat(square_size)),
                    ..Default::default()
                },
            );
        }

        if let Some(piece) = board[(x, y)] {
            if let Some(selected) = tctx.selected_square
                && selected == (x, y)
                && tctx.held
            {
                continue;
            }

            if tctx
                .animations
                .iter()
                .any(|a| a.prevent_drawing() == (x, y))
            {
                continue;
            }

            draw_texture_ex(
                tctx.get_texture(piece),
                map_x(x),
                map_y(y),
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::splat(square_size)),
                    ..Default::default()
                },
            );
        }
    }

    if let Some(square) = tctx.selected_square {
        let moves = game.all_legal_moves(square);

        for cmove in moves {
            let occupied = board[cmove].is_some();

            draw_poly(
                map_x(cmove.0) + square_size / 2.,
                map_y(cmove.1) + square_size / 2.,
                255,
                square_size / 7.,
                0.,
                if occupied {
                    Color::from_rgba(150, 0, 0, 120)
                } else {
                    Color::from_rgba(70, 70, 70, 120)
                },
            );
        }
    }

    if let Some(selected) = tctx.selected_square
        && tctx.held
        && let Some(piece) = board[selected]
    {
        let (px, py) = mouse_position();

        draw_texture_ex(
            tctx.get_texture(piece),
            px - square_size / 2.,
            py - square_size / 2.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::splat(square_size)),
                ..Default::default()
            },
        );
    }

    // render checks
    if game.is_in_check(game.turn) && tctx.animations.iter().all(|a| !a.prevent_king_decoration()) {
        let (x, y) = board.find_king(game.turn).unwrap();

        draw_poly(
            map_x(x) + square_size / 2.,
            map_y(y) + square_size / 2.,
            255,
            square_size / 3.,
            0.,
            TD_RED,
        );
    }

    if (game.is_draw() | game.is_stalemate())
        && tctx.animations.iter().all(|a| !a.prevent_king_decoration())
    {
        let (x, y) = board.find_king(game.turn).unwrap();

        draw_poly(
            map_x(x) + square_size / 2.,
            map_y(y) + square_size / 2.,
            255,
            square_size / 3.,
            0.,
            TD_GRAY,
        );
    }

    for animation in &tctx.animations {
        animation.draw(tctx);
    }
}

fn handle_input(game: &mut Game, tctx: &mut GuiGame) {
    let square_size = tctx.size / 8.0;

    // let map_x = |x: isize| x as f32 * square_size + tctx.top_left.x;
    // let map_y = |y: isize| if tctx.flipped { y as f32 } else { (7. - y as f32) } * square_size + tctx.top_left.y;

    let map_px = |x: f32| tctx.get_x(x);
    let map_py = |y: f32| tctx.get_y(y);

    let (px, py) = mouse_position();

    let x = map_px(px);
    let y = map_py(py);

    if x > 7 || y > 7 || x < 0 || y < 0 {
        return;
    }

    if is_mouse_button_pressed(MouseButton::Left)
        && let Some(square) = tctx.selected_square
        && game.is_legal_move(square, (x, y), Some(Promotion::Queen)).is_ok()
    {
        // do the move
        let effects = game.get_move_effects(square, (x, y), None);
        let mut result = game.move_checked(square, (x, y), None);

        if result == MoveResult::MissingPromotion {
            tctx.promotion = Some(
                 (square, (x, y), false)
            );
        } else {
            // play animations
            add_animations(
                &mut tctx.animations,
                effects,
                result,
                game.board.find_king(game.turn),
                false,
            );
        }
    }

    if is_mouse_button_down(MouseButton::Left) {
        if !tctx.held
            && tctx
                .animations
                .iter()
                .all(|a| a.prevent_drawing() != (x, y))
        {
            tctx.held = true;
            tctx.selected_square = Some((x, y));
        }
    } else if is_mouse_button_released(MouseButton::Left) {
        if let Some(selected) = tctx.selected_square
            && tctx.held
            && game.is_legal_move(selected, (x, y), Some(Promotion::Queen)).is_ok()
        {
            // do the move
            let effects = game.get_move_effects(selected, (x, y), None);
            let result = game.move_checked(selected, (x, y), None);

            if result == MoveResult::MissingPromotion {
                tctx.promotion = Some(
                     (selected, (x, y), true)
                );
            } else {
                // play animations
                add_animations(
                    &mut tctx.animations,
                    effects,
                    result,
                    game.board.find_king(game.turn),
                    true,
                );
            }
        }

        tctx.held = false;
    }
}

fn handle_promotion(game: &mut Game, ctx: &mut GuiGame, (from, to, skip_primary): (Pos, Pos, bool)) {
    let color = game.turn;
    const PROMOTIONS: [Promotion; 4] = [Promotion::Queen, Promotion::Knight, Promotion::Rook, Promotion::Bishop];
    ctx.held = false;

    draw_rectangle_ex(
        ctx.top_left.x, 
        ctx.top_left.y, 
        ctx.size, 
        ctx.size, 
        DrawRectangleParams {
            color: GRAY.with_alpha(0.2),
            ..Default::default()
        }
    );

    for i in 0..4 {
        let px = ctx.get_px(to.0);
        let y = if to.1 == 7 { to.1 - i } else { to.1 + i };
        let py = ctx.get_py(y);

        draw_poly(
            px + ctx.size / 16.,
            py + ctx.size / 16.,
            255,
            ctx.size / 16.,
            0.,
            GRAY,
        );

        draw_texture_ex(
            ctx.get_texture(Piece::from_promotion(PROMOTIONS[i as usize], color)),
            px,
            py,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::splat(ctx.size / 8.)),
                ..Default::default()
            },
        );
    }

    let (px, py) = mouse_position();
    let x = ctx.get_x(px);
    let y = ctx.get_y(py);

    if is_mouse_button_pressed(MouseButton::Left) && x == to.0 {
        let idx = if to.1 == 7 {
            7 - y
        } else {
            y
        };

        if (0..4).contains(&idx) {
            let promotion = Some(PROMOTIONS[idx as usize]);

            let effects = game.get_move_effects(from, to, promotion);
            let mut result = game.move_checked(from, to, promotion);

            add_animations(
                &mut ctx.animations,
                effects,
                result,
                game.board.find_king(game.turn),
                skip_primary,
            );

            ctx.promotion = None;
        }
    }
}

trait Animation {
    fn prevent_drawing(&self) -> (isize, isize);
    fn prevent_king_decoration(&self) -> bool {
        false
    }

    fn tick(&mut self, ms: f32) -> bool;
    fn draw(&self, tctx: &GuiGame);
}

fn add_animations(
    vec: &mut Vec<Box<dyn Animation>>,
    effects: Option<MoveEffects>,
    result: MoveResult,
    king_pos: Option<Pos>,
    skip_primary: bool,
) {
    if let Some(effects) = effects {
        let first = effects.piece_moves.0;

        if let Some(prom) = effects.gained_piece {
            if skip_primary {
                vec.push(Box::new(PieceFade {
                    piece: prom.1,
                    pos: first.1,
                    elapsed: 0.,
                    fade_in: false,
                    hide_piece: true
                }));

                vec.push(Box::new(PieceFade {
                    piece: prom.2,
                    pos: first.1,
                    elapsed: 0.,
                    fade_in: true,
                    hide_piece: true
                }));
            } else {
                vec.push(Box::new(PieceMoveFadeTransform {
                    start_piece: prom.1,
                    end_piece: prom.2,
                    start: first.0,
                    end: first.1,
                    elapsed: 0.,
                }));
            }
        } else if !skip_primary {
            vec.push(Box::new(PieceMove {
                piece: first.2,
                start: first.0,
                end: first.1,
                elapsed: 0.,
            }));
        }

        if let Some(second) = effects.piece_moves.1 {
            vec.push(Box::new(PieceMove {
                piece: second.2,
                start: second.0,
                end: second.1,
                elapsed: 0.,
            }));
        }

        if let Some(lost) = effects.lost_piece {
            vec.push(Box::new(PieceFade {
                piece: lost.1,
                pos: lost.0,
                elapsed: 0.,
                fade_in: false,
                hide_piece: false
            }));
        }

        if let Some(pos) = king_pos && matches!(result, MoveResult::Check | MoveResult::Checkmate) {
            vec.push(Box::new(DecorationAnim {
                pos,
                elapsed: 0.,
                final_color: TD_RED
            }));
        }

        if let Some(pos) = king_pos && matches!(result, MoveResult::Draw | MoveResult::Stalemate) {
            vec.push(Box::new(DecorationAnim {
                pos,
                elapsed: 0.,
                final_color: TD_GRAY
            }));
        }
    }
}

struct PieceMove {
    piece: Piece,
    start: (isize, isize),
    end: (isize, isize),
    elapsed: f32,
}

impl PieceMove {
    const ANIMATION_TIME: f32 = 0.150;

    fn easing(&self) -> f32 {
        let x = self.elapsed / Self::ANIMATION_TIME;
        // simplest ease function is just 'x'
        let ease = f32::sqrt(1. - f32::powi(x - 1., 2));
        f32::min(ease, 1.)
    }
}

impl Animation for PieceMove {
    fn prevent_drawing(&self) -> (isize, isize) {
        self.end
    }

    fn tick(&mut self, ms: f32) -> bool {
        self.elapsed += ms;
        self.elapsed >= Self::ANIMATION_TIME
    }

    fn draw(&self, tctx: &GuiGame) {
        let prog = self.easing();

        let sx = tctx.get_px(self.start.0);
        let sy = tctx.get_py(self.start.1);

        let ex = tctx.get_px(self.end.0);
        let ey = tctx.get_py(self.end.1);

        draw_texture_ex(
            tctx.get_texture(self.piece),
            prog * ex + (1. - prog) * sx,
            prog * ey + (1. - prog) * sy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::splat(tctx.size / 8.)),
                ..Default::default()
            },
        );
    }
}

struct PieceFade {
    piece: Piece,
    pos: (isize, isize),
    elapsed: f32,
    fade_in: bool,
    hide_piece: bool
}

impl PieceFade {
    const ANIMATION_TIME: f32 = 0.150;

    fn easing(&self) -> f32 {
        let x = self.elapsed / Self::ANIMATION_TIME;
        // https://easings.net/
        let ease = 1. - (1. - x) * (1. - x);
        f32::min(ease, 1.)
    }
}

impl Animation for PieceFade {
    fn prevent_drawing(&self) -> (isize, isize) {
        if self.hide_piece { self.pos } else { (-1, -1) }
    }

    fn tick(&mut self, ms: f32) -> bool {
        self.elapsed += ms;
        self.elapsed >= Self::ANIMATION_TIME
    }

    fn draw(&self, tctx: &GuiGame) {
        let prog = if self.fade_in {
            self.easing()
        } else {
            1. - self.easing()
        };

        let sx = tctx.get_px(self.pos.0);
        let sy = tctx.get_py(self.pos.1);

        draw_texture_ex(
            tctx.get_texture(self.piece),
            sx,
            sy,
            WHITE.with_alpha(prog),
            DrawTextureParams {
                dest_size: Some(Vec2::splat(tctx.size / 8.)),
                ..Default::default()
            },
        );
    }
}

struct DecorationAnim {
    pos: (isize, isize),
    elapsed: f32,
    final_color: Color,
}

impl DecorationAnim {
    const ANIMATION_TIME: f32 = 0.150;

    fn easing(&self) -> f32 {
        let x = self.elapsed / Self::ANIMATION_TIME;
        // https://easings.net/
        let ease = 1. - (1. - x) * (1. - x);
        f32::min(ease, 1.)
    }
}

impl Animation for DecorationAnim {
    fn prevent_drawing(&self) -> (isize, isize) {
        (-1, -1)
    }

    fn prevent_king_decoration(&self) -> bool {
        true
    }

    fn tick(&mut self, ms: f32) -> bool {
        self.elapsed += ms;
        self.elapsed >= Self::ANIMATION_TIME
    }

    fn draw(&self, tctx: &GuiGame) {
        let prog = self.easing();

        let sx = tctx.get_px(self.pos.0);
        let sy = tctx.get_py(self.pos.1);

        let color = self.final_color.with_alpha(self.final_color.a * prog);

        draw_poly(
            sx + tctx.size / 16.,
            sy + tctx.size / 16.,
            255,
            tctx.size / 24.,
            0.,
            color,
        );
    }
}

struct PieceMoveFadeTransform {
    start_piece: Piece,
    end_piece: Piece,
    start: (isize, isize),
    end: (isize, isize),
    elapsed: f32,
}

impl PieceMoveFadeTransform {
    const ANIMATION_TIME: f32 = 0.150;

    fn easing_move(&self) -> f32 {
        let x = self.elapsed / Self::ANIMATION_TIME;
        let ease = f32::sqrt(1. - f32::powi(x - 1., 2));
        f32::min(ease, 1.)
    }

    fn easing_fade(&self) -> f32 {
        let x = self.elapsed / Self::ANIMATION_TIME;
        let ease = 1. - (1. - x) * (1. - x);
        f32::min(ease, 1.)
    }
}

impl Animation for PieceMoveFadeTransform {
    fn prevent_drawing(&self) -> (isize, isize) {
        self.end
    }

    fn tick(&mut self, ms: f32) -> bool {
        self.elapsed += ms;
        self.elapsed >= Self::ANIMATION_TIME
    }

    fn draw(&self, tctx: &GuiGame) {
        let pm = self.easing_move();
        let pf = self.easing_fade();

        let sx = tctx.get_px(self.start.0);
        let sy = tctx.get_py(self.start.1);

        let ex = tctx.get_px(self.end.0);
        let ey = tctx.get_py(self.end.1);

        draw_texture_ex(
            tctx.get_texture(self.start_piece),
            pm * ex + (1. - pm) * sx,
            pm * ey + (1. - pm) * sy,
            WHITE.with_alpha(1. - pf),
            DrawTextureParams {
                dest_size: Some(Vec2::splat(tctx.size / 8.)),
                ..Default::default()
            },
        );

        draw_texture_ex(
            tctx.get_texture(self.end_piece),
            pm * ex + (1. - pm) * sx,
            pm * ey + (1. - pm) * sy,
            WHITE.with_alpha(pf),
            DrawTextureParams {
                dest_size: Some(Vec2::splat(tctx.size / 8.)),
                ..Default::default()
            },
        );
    }
}
