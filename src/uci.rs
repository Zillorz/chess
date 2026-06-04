#![allow(unused)]
use std::io::{BufRead, BufReader, Write};
use std::num::{NonZeroU8, NonZeroU64};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use crate::chess::Pos;
use crate::{Game, chess::Promotion};
use std::time::{Duration, Instant};

use std::fmt::Write as _;

pub struct ThreadedUci {
    sender: Sender<Message>,
    receiver: Receiver<ResultMessage>,
    // handle: JoinHandle<()>
}

pub(crate) enum Message {
    RecommendMove(Game, Limits),
}

pub(crate) enum ResultMessage {
    Result((Pos, Pos, Option<Promotion>, String)),
}

impl ThreadedUci {
    pub(crate) fn new() -> Self {
        let (s, rx) = std::sync::mpsc::channel();
        let (s2, rx2) = std::sync::mpsc::channel();

        let _thread = std::thread::spawn(move || {
            let mut uci = Uci::new();

            while let Ok(message) = rx.recv() {
                match message {
                    Message::RecommendMove(game, limits) => {
                        let ret = uci.recommend_move(&game, limits);
                        s2.send(ResultMessage::Result(ret)).unwrap();
                    }
                }
            }
        });

        Self {
            sender: s,
            // handle: thread,
            receiver: rx2,
        }
    }

    pub(crate) fn new_delay(min_time: Duration) -> Self {
        let (s, rx) = std::sync::mpsc::channel();
        let (s2, rx2) = std::sync::mpsc::channel();

        let _thread = std::thread::spawn(move || {
            let mut uci = Uci::new();

            while let Ok(message) = rx.recv() {
                match message {
                    Message::RecommendMove(game, limits) => {
                        let time = Instant::now();
                        let ret = uci.recommend_move(&game, limits);

                        if let Some(duration) = min_time.checked_sub(time.elapsed()) {
                            std::thread::sleep(duration);
                        }

                        s2.send(ResultMessage::Result(ret)).unwrap();
                    }
                }
            }
        });

        Self {
            sender: s,
            // handle: thread,
            receiver: rx2,
        }
    }

    pub(crate) fn recommend_move(&self, game: Game, limits: Limits) {
        self.sender
            .send(Message::RecommendMove(game, limits))
            .unwrap();
    }

    pub(crate) fn try_result(&self) -> Option<(Pos, Pos, Option<Promotion>, String)> {
        if let Ok(ResultMessage::Result(ret)) = self.receiver.try_recv() {
            return Some(ret);
        }

        None
    }
}

pub struct Uci {
    process: Child,
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

impl Uci {
    #[cfg(windows)]
    pub(crate) fn new() -> Self {
        let mut child = Command::new("cmd")
            .args(["/C", "uci.bat"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();

        writeln!(child.stdin.as_mut().unwrap(), "uci").unwrap();
        Uci { process: child }
    }

    #[cfg(unix)]
    pub(crate) fn new() -> Self {
        let mut child = Command::new("bash")
            .args(["uci.sh"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();

        writeln!(child.stdin.as_mut().unwrap(), "uci").unwrap();
        Uci { process: child }
    }

    pub(crate) fn recommend_move(
        &mut self,
        game: &Game,
        limits: Limits,
    ) -> (Pos, Pos, Option<Promotion>, String) {
        let stdin = self.process.stdin.as_mut().unwrap();
        let fen = game.as_fen();

        writeln!(stdin, "position fen {fen}").unwrap();
        writeln!(stdin, "go {}", limits.into_limit_string()).unwrap();

        let mut stdout = BufReader::new(self.process.stdout.as_mut().unwrap());

        loop {
            let mut string = String::new();
            stdout.read_line(&mut string).unwrap();

            if string.starts_with("bestmove") {
                let mut parts = string.split(' ');

                let alg_move = parts.nth(1).unwrap();
                let mut iter = alg_move.chars();

                let x1 = iter.next().unwrap() as isize - 'a' as isize;
                let y1 = iter.next().unwrap() as isize - '1' as isize;

                let x2 = iter.next().unwrap() as isize - 'a' as isize;
                let y2 = iter.next().unwrap() as isize - '1' as isize;

                let promotion = if let Some(p) = iter.next() {
                    match p {
                        'q' => Some(Promotion::Queen),
                        'n' => Some(Promotion::Knight),
                        'r' => Some(Promotion::Rook),
                        'b' => Some(Promotion::Bishop),
                        '+' | '#' => None,
                        c => {
                            eprintln!("Unknown promotion letter, '{c}'");
                            None
                        }
                    }
                } else {
                    None
                };

                return ((x1, y1), (x2, y2), promotion, alg_move.to_string());
            }
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct Limits {
    time: Option<NonZeroU64>,
    depth: Option<NonZeroU8>,
    w_time: Option<NonZeroU64>,
    b_time: Option<NonZeroU64>,
    w_inc: Option<NonZeroU64>,
    b_inc: Option<NonZeroU64>,
}

impl Limits {
    pub fn time(mut self, time: u64) -> Self {
        self.time = NonZeroU64::new(time);
        self
    }

    pub fn depth(mut self, depth: u8) -> Self {
        self.depth = NonZeroU8::new(depth);
        self
    }

    pub fn w_time(mut self, w_time: u64) -> Self {
        self.w_time = NonZeroU64::new(w_time);
        self
    }

    pub fn b_time(mut self, b_time: u64) -> Self {
        self.b_time = NonZeroU64::new(b_time);
        self
    }

    pub fn set_time(&mut self, w_time: u64, b_time: u64) {
        self.w_time = NonZeroU64::new(w_time);
        self.b_time = NonZeroU64::new(b_time);
    }

    pub fn w_inc(mut self, w_inc: u64) -> Self {
        self.w_inc = NonZeroU64::new(w_inc);
        self
    }

    pub fn b_inc(mut self, b_inc: u64) -> Self {
        self.b_inc = NonZeroU64::new(b_inc);
        self
    }

    fn into_limit_string(self) -> String {
        let mut ret = String::new();

        if let Some(time) = self.time {
            write!(ret, " movetime {time}");
        }

        if let Some(depth) = self.depth {
            write!(ret, " depth {depth}");
        }

        if let Some(w_time) = self.w_time {
            write!(ret, " wtime {w_time}");
        }

        if let Some(b_time) = self.b_time {
            write!(ret, " btime {b_time}");
        }

        if let Some(w_inc) = self.w_inc {
            write!(ret, " winc {w_inc}");
        }

        if let Some(b_inc) = self.b_inc {
            write!(ret, " binc {b_inc}");
        }

        // default limit will be depth 20
        if ret.is_empty() {
            ret.push_str("depth 20");
        }

        ret
    }
}
