mod parse;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use fas_rs_fw::Fps;

pub(crate) type GameList = HashMap<String, u32>;

pub struct Config {
    game_list: Arc<Mutex<GameList>>,
    pause: Arc<AtomicBool>,
}

impl Drop for Config {
    fn drop(&mut self) {
        self.pause.store(true, Ordering::Release);
    }
}

impl Config {
    pub fn new(path: PathBuf) -> Self {
        let game_list = Arc::new(Mutex::new(GameList::new()));
        let game_list_clone = game_list.clone();

        let pause = Arc::new(AtomicBool::new(false));
        let pause_clone = pause.clone();

        thread::spawn(move || parse::wait_and_parse(path, game_list_clone, pause_clone));

        Self { game_list, pause }
    }

    pub fn cur_game_fps(&self) -> Option<(String, Fps)> {
        let list = self.game_list.lock().unwrap();

        let pkgs = Self::get_top_pkgname()?;
        let pkg = pkgs.into_iter().find(|key| list.contains_key(key))?;

        let (game, fps) = list.get_key_value(&pkg)?;
        Some((game.to_owned(), fps.to_owned()))
    }

    fn get_top_pkgname() -> Option<HashSet<String>> {
        let dump = Command::new("dumpsys")
            .args(["window", "visible-apps"])
            .output()
            .ok()?;
        let dump = String::from_utf8_lossy(&dump.stdout).into_owned();

        Some(
            dump.lines()
                .filter(|l| l.contains("package="))
                .map(|p| {
                    p.split_whitespace()
                        .nth(2)
                        .and_then(|p| p.split('=').nth(1))
                        .unwrap()
                })
                .zip(
                    dump.lines()
                        .filter(|l| l.contains("canReceiveKeys()"))
                        .map(|k| k.contains("canReceiveKeys()=true")),
                )
                .filter(|(_, k)| *k)
                .map(|(p, _)| p.to_owned())
                .collect(),
        )
    }
}
