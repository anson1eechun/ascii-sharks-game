use std::fmt::Display;
use std::ops::RangeInclusive;
use std::process::exit;
use std::str::FromStr;
use std::sync::OnceLock;

use crate::screen::ColorMode;
use crate::term;
use crate::world::SharkKind;

const HELP_EN: &str = "sharks-game — eat fish, grow, and become the biggest shark in the sea

Usage: sharks-game [options]

Game
  -k, --shark <kind>       Your shark (default: random; -l lists them)
  -o, --others <kind,...>  The other sharks (default: random kinds)
  -n, --count <n>          How many other sharks, 0–11 (default 3)
      --fish <n>           How many fish, 1–1000 (default: by window size)
      --seed <n>           Play the same sea again
      --speed <x>          Game speed, 0.1–5 (default 1)
  -l, --list               List the shark kinds

Display
  -f, --fps <n>            Frames per second, 1–120 (default 30)
      --color <mode>       auto, truecolor, 256, 16 or mono (default auto)
      --ascii              ASCII glyphs only
      --aspect <n>         Cell height / width (default: ask the terminal)
      --calm               No shimmer or flicker on the sharks
  -V, --version
  -h, --help

Controls
  arrows, WASD, hjkl  swim that way
  mouse               your shark follows the pointer
  space, click        dash
  p                   pause
  Ctrl-Z              suspend
  q, Esc              quit

Messages follow the locale; set SHARKS_LANG=zh-TW for Chinese.";

const HELP_ZH: &str = "sharks-game — 吃魚、長大，成為這片海裡最大的鯊魚

用法: sharks-game [選項]

遊戲
  -k, --shark <種類>       你的鯊魚（預設隨機；用 -l 看清單）
  -o, --others <種類,...>  其他鯊魚（預設隨機挑選）
  -n, --count <n>          其他鯊魚的數量，0～11（預設 3）
      --fish <n>           魚的數量，1～1000（預設依視窗大小）
      --seed <n>           重玩同一片海
      --speed <x>          遊戲速度，0.1～5（預設 1）
  -l, --list               列出所有鯊魚種類

顯示
  -f, --fps <n>            每秒畫面數，1～120（預設 30）
      --color <模式>       auto、truecolor、256、16 或 mono（預設 auto）
      --ascii              只用 ASCII 字元
      --aspect <n>         字元格高寬比（預設向終端機查詢）
      --calm               鯊魚不閃爍、不變色
  -V, --version
  -h, --help

操作
  方向鍵、WASD、hjkl  往那個方向游
  滑鼠                你的鯊魚跟著游標游
  空白鍵、點擊        衝刺
  p                   暫停
  Ctrl-Z              暫停到背景
  q、Esc              離開

訊息語言依語系而定；設定 SHARKS_LANG=en 可改用英文。";

pub fn zh() -> bool {
    static ZH: OnceLock<bool> = OnceLock::new();
    *ZH.get_or_init(|| {
        ["SHARKS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
            .is_some_and(|v| v.to_ascii_lowercase().starts_with("zh"))
    })
}

pub fn t(en: &'static str, zh_text: &'static str) -> &'static str {
    if zh() { zh_text } else { en }
}

pub fn fail(msg: &str) -> ! {
    eprintln!("sharks-game: {msg}");
    exit(2);
}

pub const MAX_OTHERS: usize = 11;

#[derive(Debug)]
pub struct Options {
    pub shark: Option<SharkKind>,
    pub others: Option<Vec<SharkKind>>,
    pub count: usize,
    pub fish: Option<usize>,
    pub seed: Option<u32>,
    pub speed: f64,
    pub calm: bool,
    pub fps: f64,
    pub color: Option<ColorMode>,
    pub ascii: bool,
    pub aspect: Option<f64>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            shark: None,
            others: None,
            count: 3,
            fish: None,
            seed: None,
            speed: 1.0,
            calm: false,
            fps: 30.0,
            color: None,
            ascii: false,
            aspect: None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Stop {
    Print(String),
    Fail(String),
}

fn number<T: FromStr + PartialOrd + Display>(
    flag: &str,
    value: &str,
    range: RangeInclusive<T>,
) -> Result<T, Stop> {
    match value.parse() {
        Ok(n) if range.contains(&n) => Ok(n),
        _ => Err(Stop::Fail(format!(
            "{flag} {} {} {} {}",
            t("must be between", "要介於"),
            range.start(),
            t("and", "到"),
            range.end()
        ))),
    }
}

pub fn parse() -> Options {
    parse_from(std::env::args().skip(1)).unwrap_or_else(|stop| match stop {
        Stop::Print(text) => {
            println!("{text}");
            exit(0);
        }
        Stop::Fail(msg) => fail(&msg),
    })
}

pub fn parse_from(args: impl IntoIterator<Item = String>) -> Result<Options, Stop> {
    let mut o = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, mut inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_owned(), Some(v.to_owned())),
            _ => (arg, None),
        };
        let mut value = || {
            inline
                .take()
                .or_else(|| args.next())
                .ok_or_else(|| Stop::Fail(format!("{flag} {}", t("needs a value", "需要一個值"))))
        };
        match flag.as_str() {
            "-h" | "--help" => return Err(Stop::Print(t(HELP_EN, HELP_ZH).into())),
            "-V" | "--version" => {
                return Err(Stop::Print(format!(
                    "sharks-game {}",
                    env!("CARGO_PKG_VERSION")
                )));
            }
            "-l" | "--list" => {
                let list = SharkKind::ALL.map(|k| format!("{:<12} {}", k.name(), k.label(zh())));
                return Err(Stop::Print(list.join("\n")));
            }
            "-k" | "--shark" => {
                let name = value()?;
                o.shark = Some(SharkKind::parse(&name).ok_or_else(|| unknown_kind(&name))?);
            }
            "-o" | "--others" => o.others = Some(parse_kinds(&flag, &value()?)?),
            "-n" | "--count" => o.count = number(&flag, &value()?, 0..=MAX_OTHERS)?,
            "--fish" => o.fish = Some(number(&flag, &value()?, 1..=1000)?),
            "--seed" => o.seed = Some(number(&flag, &value()?, 0..=u32::MAX)?),
            "--speed" => o.speed = number(&flag, &value()?, 0.1..=5.0)?,
            "--calm" => o.calm = true,
            "-f" | "--fps" => o.fps = number(&flag, &value()?, 1.0..=120.0)?,
            "--color" => {
                let v = value()?;
                o.color = match v.as_str() {
                    "auto" => None,
                    v => Some(ColorMode::parse(v).ok_or_else(|| {
                        Stop::Fail(format!(
                            "--color: auto, truecolor, 256, 16 {} mono ({v}?)",
                            t("or", "或")
                        ))
                    })?),
                };
            }
            "--ascii" => o.ascii = true,
            "--aspect" => o.aspect = Some(number(&flag, &value()?, term::ASPECT)?),
            _ if zh() => return Err(Stop::Fail(format!("不認得的選項：{flag}（用 -h 看說明）"))),
            _ => return Err(Stop::Fail(format!("unknown option: {flag} (see -h)"))),
        }
    }
    Ok(o)
}

fn unknown_kind(names: &str) -> Stop {
    Stop::Fail(if zh() {
        format!("不認得的鯊魚種類：{names}（用 -l 看清單）")
    } else {
        format!("unknown shark kind: {names} (see -l)")
    })
}

pub fn parse_kinds(flag: &str, list: &str) -> Result<Vec<SharkKind>, Stop> {
    let mut kinds = Vec::new();
    let mut unknown = Vec::new();
    for name in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match SharkKind::parse(name) {
            Some(kind) => kinds.push(kind),
            None => unknown.push(name),
        }
    }
    if !unknown.is_empty() {
        return Err(unknown_kind(&unknown.join(", ")));
    }
    if !(1..=MAX_OTHERS).contains(&kinds.len()) {
        return Err(Stop::Fail(format!(
            "{flag}: 1–{MAX_OTHERS} {}",
            t("sharks", "隻鯊魚")
        )));
    }
    Ok(kinds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, Stop> {
        parse_from(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn picks_your_shark_and_the_others() {
        let o = parse(&["-k", "mako", "--others=tiger,zebra", "-n", "0"]).unwrap();
        assert_eq!(o.shark, Some(SharkKind::Mako));
        assert_eq!(o.others, Some(vec![SharkKind::Tiger, SharkKind::Zebra]));
        assert_eq!(o.count, 0);
        assert_eq!(parse(&[]).unwrap().count, 3);
    }

    #[test]
    fn rejects_bad_values() {
        for args in [
            &["-k", "whale"][..],
            &["-k", "tiger,mako"],
            &["-o", ""],
            &["-o", ","],
            &["-o", "tiger,whale"],
            &["-n", "12"],
            &["--fps", "0"],
            &["--color", "rgb"],
            &["-f"],
            &["--clock"],
        ] {
            assert!(matches!(parse(args), Err(Stop::Fail(_))), "{args:?}");
        }
        assert!(parse(&["-o", &["mako"; 11].join(",")]).is_ok());
        assert!(parse(&["-o", &["mako"; 12].join(",")]).is_err());
    }

    #[test]
    fn colour_auto_and_modes() {
        assert_eq!(parse(&["--color", "auto"]).unwrap().color, None);
        assert_eq!(
            parse(&["--color=256"]).unwrap().color,
            Some(ColorMode::Ansi256)
        );
    }
}
