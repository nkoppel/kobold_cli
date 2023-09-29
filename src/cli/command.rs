use super::*;
use std::str::FromStr;

pub enum Command {
    Help,
    Exit,
    Reload,
    Load(String),
    Char(String),
    Gen,
    Swipe,
    Undo,
    Redo,
    SwipeList,
    SwipeIndex(usize),
}

impl FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut words = s.split_whitespace();

        let out = match words.next() {
            None => bail!("Cannot parse command from empty input"),
            Some("help") => Command::Help,
            Some("exit") => Command::Exit,
            Some("load") => {
                if let Some(name) = words.next() {
                    Command::Load(name.to_string())
                } else {
                    Command::Reload
                }
            }
            Some("reload") => Command::Reload,
            Some("char") => {
                let Some(space) = s.find(' ') else {
                    bail!("\"char\" command requires a name argument");
                };

                let name = s[space..].trim();

                if name.is_empty() {
                    bail!("\"char\" command requires a name argument");
                }

                Command::Char(name.to_string())
            }
            Some("swipe") => match words.next() {
                None => Command::Swipe,
                Some("list") => Command::SwipeList,
                Some(s) => {
                    if let Ok(i) = s.parse::<usize>() {
                        Command::SwipeIndex(i)
                    } else {
                        bail!("Unrecognized subcommand for swipe: {s:?}")
                    }
                }
            },
            Some("gen") => Command::Gen,
            Some("regen") => Command::Swipe,
            Some("undo") => Command::Undo,
            Some("redo") => Command::Redo,
            Some(c) => {
                bail!("Unrecognized command: {c:?}. Type \"help\" to view a list of commands.")
            }
        };

        Ok(out)
    }
}
