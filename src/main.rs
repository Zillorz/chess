#![warn(clippy::pedantic)]
// #![windows_subsystem = "windows"]

mod chess;
mod uci;

use macroquad::audio::{Sound, load_sound, play_sound_once};
use macroquad::{Error, hash};
use std::collections::HashMap;

use crate::chess::{BoardIter, Game, MoveEffects, MoveResult, Piece, Pos, Promotion};
use crate::uci::{Limits, ThreadedUci};
use macroquad::prelude::*;
use macroquad::ui::{Skin, root_ui};

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

    let checkbox_style = root_ui()
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

#[allow(clippy::struct_excessive_bools)]
struct GuiGame {
    piece_textures: HashMap<Piece, Texture2D>,
    sounds: HashMap<ChessSound, Sound>,
    light_square_texture: Texture2D,
    dark_square_texture: Texture2D,

    animations: Vec<Box<dyn Animation>>,

    top_left: Vec2,
    size: f32,
    flipped: bool,

    two_player: bool,
    player_color: chess::Color,
    uci: Option<ThreadedUci>,

    complete: bool,

    selected_square: Option<Pos>,
    held: bool,

    // from, to, skip_primary_animation
    promotion: Option<(Pos, Pos, bool)>,
}

impl GuiGame {
    async fn load_from_files(
        two_player: bool,
        player_color: chess::Color,
        flipped: bool,
    ) -> Result<Self, Error> {
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

        let uci = if two_player {
            None
        } else {
            Some(ThreadedUci::new())
        };

        Ok(GuiGame {
            piece_textures,
            sounds,
            light_square_texture,
            dark_square_texture,

            animations: Vec::new(),

            top_left: Vec2::splat(0.),
            size: 1024.,
            flipped,

            two_player,
            player_color,
            uci,

            complete: false,

            selected_square: None,
            held: false,

            promotion: None,
        })
    }

    fn get_texture(&self, piece: Piece) -> &Texture2D {
        self.piece_textures.get(&piece).unwrap()
    }

    #[allow(clippy::cast_precision_loss)]
    fn get_px(&self, x: isize) -> f32 {
        x as f32 * (self.size / 8.) + self.top_left.x
    }

    #[allow(clippy::cast_precision_loss)]
    fn get_py(&self, y: isize) -> f32 {
        (if self.flipped {
            y as f32
        } else {
            7. - y as f32
        }) * (self.size / 8.)
            + self.top_left.y
    }

    #[allow(clippy::cast_possible_truncation)]
    fn get_x(&self, px: f32) -> isize {
        ((px - self.top_left.x) / (self.size / 8.)) as isize
    }

    #[allow(clippy::cast_possible_truncation)]
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
    let mut ctx = GuiGame::load_from_files(two_player, player_color, flipped)
        .await
        .unwrap();

    request_new_screen_size(1024.0, 1024.0);
    next_frame().await;

    loop {
        let size = f32::min(screen_height(), screen_width());
        ctx.size = size;
        if let Some(uci) = &ctx.uci
            && let Some((from, to, prom, _)) = uci.try_result()
            && !ctx.two_player
            && game.turn != ctx.player_color
        {
            make_move(&mut game, &mut ctx, from, to, prom, false);
        }

        ctx.animations
            .retain_mut(|animation| !animation.tick(get_frame_time()));

        render(&game, &ctx);

        if !ctx.complete {
            if let Some(promotion) = ctx.promotion {
                handle_promotion(&mut game, &mut ctx, promotion);
            } else {
                handle_input(&mut game, &mut ctx);
            }
        }

        next_frame().await;
    }
}

fn render(game: &Game, ctx: &GuiGame) {
    let board = game.board;
    let square_size = ctx.size / 8.0;

    for (x, y) in BoardIter::default() {
        let is_dark_square = (x + y) % 2 == 0;

        let px = ctx.get_px(x);
        let py = ctx.get_py(y);

        // draw square
        draw_texture_ex(
            if is_dark_square {
                &ctx.dark_square_texture
            } else {
                &ctx.light_square_texture
            },
            px,
            py,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::splat(square_size)),
                ..Default::default()
            },
        );

        if let Some(piece) = board[(x, y)] {
            // Don't render held pieces
            let held = ctx.selected_square.is_some_and(|s| s == (x, y)) && ctx.held;
            let animated = ctx.animations.iter().any(|a| a.prevent_drawing() == (x, y));

            if !held && !animated {
                draw_texture_ex(
                    ctx.get_texture(piece),
                    px,
                    py,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::splat(square_size)),
                        ..Default::default()
                    },
                );
            }
        }
    }

    // if selected, show legal moves
    if let Some(square) = ctx.selected_square {
        let moves = game.all_legal_moves(square);

        for (dx, dy) in moves {
            let occupied = board[(dx, dy)].is_some();

            draw_poly(
                ctx.get_px(dx) + square_size / 2.,
                ctx.get_py(dy) + square_size / 2.,
                255,
                square_size / 7.,
                0.,
                if occupied { TD_RED } else { TD_GRAY },
            );
        }
    }

    // draw held piece at mouse
    if let Some(selected) = ctx.selected_square
        && let Some(piece) = board[selected]
        && ctx.held
    {
        let (px, py) = mouse_position();

        draw_texture_ex(
            ctx.get_texture(piece),
            px - square_size / 2.,
            py - square_size / 2.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::splat(square_size)),
                ..Default::default()
            },
        );
    }

    // render checks, unless animation is present
    if game.is_in_check(game.turn) && ctx.animations.iter().all(|a| !a.prevent_king_decoration()) {
        let (x, y) = board.find_king(game.turn).unwrap();

        draw_poly(
            ctx.get_px(x) + square_size / 2.,
            ctx.get_py(y) + square_size / 2.,
            255,
            square_size / 3.,
            0.,
            TD_RED,
        );
    }

    // render draw, unless animation is present
    if (game.is_draw() | game.is_stalemate())
        && ctx.animations.iter().all(|a| !a.prevent_king_decoration())
    {
        let (x, y) = board.find_king(game.turn).unwrap();

        draw_poly(
            ctx.get_px(x) + square_size / 2.,
            ctx.get_py(y) + square_size / 2.,
            255,
            square_size / 3.,
            0.,
            TD_GRAY,
        );
    }

    // draw the animations
    for animation in &ctx.animations {
        animation.draw(ctx);
    }
}

fn handle_input(game: &mut Game, ctx: &mut GuiGame) {
    let (px, py) = mouse_position();

    let x = ctx.get_x(px);
    let y = ctx.get_y(py);

    if x > 7 || y > 7 || x < 0 || y < 0 {
        return;
    }

    // if clicked, a square is selected, and the move (selected->click location) is legal
    if is_mouse_button_pressed(MouseButton::Left)
        && let Some(selected) = ctx.selected_square
        && game
            .is_legal_move(selected, (x, y), Some(Promotion::Queen))
            .is_ok()
    {
        make_move(game, ctx, selected, (x, y), None, false);
        ctx.selected_square = None;
    }

    // if the mouse button is held and no piece is held, hold a piece
    if is_mouse_button_down(MouseButton::Left) {
        if !ctx.held
            && (ctx.two_player || ctx.player_color == game.turn)
            && ctx.animations.iter().all(|a| a.prevent_drawing() != (x, y))
        {
            ctx.held = true;
            ctx.selected_square = Some((x, y));
        }
    }
    // if a piece is dragged, play the move
    else if is_mouse_button_released(MouseButton::Left) {
        if let Some(selected) = ctx.selected_square
            && ctx.held
            && game
                .is_legal_move(selected, (x, y), Some(Promotion::Queen))
                .is_ok()
        {
            make_move(game, ctx, selected,(x, y), None, true);
            ctx.selected_square = None;
        }

        ctx.held = false;
    }
}

// Transfers to promotion mode in case of promotion when None is passed in
fn make_move(game: &mut Game, ctx: &mut GuiGame, from: Pos, to: Pos, promotion: Option<Promotion>, skip_primary: bool) {
    // get the effects
    let effects = game.get_move_effects(from, to, promotion);
    let result = game.move_checked(from, to, promotion);

    // do the move, changing to promotion state if necessary
    if result == MoveResult::MissingPromotion {
        ctx.promotion = Some((from, to, skip_primary));
    } else {
        play_sounds(ctx, effects.as_ref(), result);
        add_animations(
            &mut ctx.animations,
            effects.as_ref(),
            result,
            game.board.find_king(game.turn),
            skip_primary,
        );

        if let Some(uci) = &ctx.uci
            && !ctx.two_player
            && game.turn != ctx.player_color
        {
            uci.recommend_move(*game, Limits::default());
        }
    }
}

const PROMOTIONS: [Promotion; 4] = [
    Promotion::Queen,
    Promotion::Knight,
    Promotion::Rook,
    Promotion::Bishop,
];

fn handle_promotion(
    game: &mut Game,
    ctx: &mut GuiGame,
    (from, to, skip_primary): (Pos, Pos, bool),
) {
    let color = game.turn;
    let square_size = ctx.size / 8.;
    let (dx, dy) = to;

    ctx.held = false;

    draw_rectangle_ex(
        ctx.top_left.x,
        ctx.top_left.y,
        ctx.size,
        ctx.size,
        DrawRectangleParams {
            color: GRAY.with_alpha(0.2),
            ..Default::default()
        },
    );

    for i in 0..4 {
        let px = ctx.get_px(dx);
        let y = if dy == 7 { 7 - i } else { i };
        let py = ctx.get_py(y);

        draw_poly(
            px + square_size / 2.,
            py + square_size / 2.,
            255,
            square_size / 2.,
            0.,
            GRAY,
        );

        draw_texture_ex(
            ctx.get_texture(Piece::from_promotion(PROMOTIONS[i.cast_unsigned()], color)),
            px,
            py,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::splat(square_size)),
                ..Default::default()
            },
        );
    }

    let (px, py) = mouse_position();
    let x = ctx.get_x(px);
    let y = ctx.get_y(py);

    if is_mouse_button_pressed(MouseButton::Left) && x == to.0 {
        let idx = if dy == 7 { 7 - y } else { y };

        if (0..4).contains(&idx) {
            let promotion = Some(PROMOTIONS[idx.cast_unsigned()]);

            make_move(game, ctx, from, to, promotion, skip_primary);
            ctx.promotion = None;
        }
    }
}

fn play_sounds(ctx: &GuiGame, effects: Option<&MoveEffects>, result: MoveResult) {
    // check has the highest priority
    if let Some(check) = ctx.sounds.get(&ChessSound::Check)
        && matches!(result, MoveResult::Check | MoveResult::Checkmate)
    {
        play_sound_once(check);
        return;
    }

    // castle = capture > move
    if let Some(effects) = effects {
        if let Some(capture) = ctx.sounds.get(&ChessSound::Capture)
            && effects.lost_piece.is_some()
        {
            play_sound_once(capture);
            return;
        } else if let Some(castle) = ctx.sounds.get(&ChessSound::Castle)
            && effects.piece_moves.1.is_some()
        {
            play_sound_once(castle);
            return;
        }
    }

    if let Some(default) = ctx.sounds.get(&ChessSound::Move) {
        play_sound_once(default);
    }
}

// decide which animations to add to the queue
fn add_animations(
    vec: &mut Vec<Box<dyn Animation>>,
    effects: Option<&MoveEffects>,
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
                    hide_piece: true,
                }));

                vec.push(Box::new(PieceFade {
                    piece: prom.2,
                    pos: first.1,
                    elapsed: 0.,
                    fade_in: true,
                    hide_piece: true,
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
                hide_piece: false,
            }));
        }

        if let Some(pos) = king_pos
            && matches!(result, MoveResult::Check | MoveResult::Checkmate)
        {
            vec.push(Box::new(DecorationAnim {
                pos,
                elapsed: 0.,
                final_color: TD_RED,
            }));
        }

        if let Some(pos) = king_pos
            && matches!(result, MoveResult::Draw | MoveResult::Stalemate)
        {
            vec.push(Box::new(DecorationAnim {
                pos,
                elapsed: 0.,
                final_color: TD_GRAY,
            }));
        }
    }
}

trait Animation {
    // stops drawing piece at pos, pass (-1, -1) to draw all pieces
    fn prevent_drawing(&self) -> Pos;

    // stops drawing king decorations (check/draw circles) if true, default: false
    fn prevent_king_decoration(&self) -> bool {
        false
    }

    // tick in time, update animation state
    fn tick(&mut self, ms: f32) -> bool;

    // draw animation
    fn draw(&self, tctx: &GuiGame);
}

// easing is a helper function in ALL of these animations
// it is a function that is between 0 and 1, representing how far along the animation
// should be. Implemented so animations aren't all linear

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
    hide_piece: bool,
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
