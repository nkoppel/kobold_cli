mod command;
mod history;

use command::*;
use history::*;

use crate::files::*;
use crate::server::*;
use anyhow::{bail, Result};

use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Cli {
    servers: Option<Servers>,
    file: Option<PathBuf>,
    prompt: Option<Prompt>,
    character: Option<String>,
    history: History,
}

impl Cli {
    pub fn new() -> Cli {
        Self::default()
    }

    fn get_file(&self) -> Result<&PathBuf> {
        let Some(file) = self.file.as_ref() else {
            bail!("Can't reload: No file loaded!")
        };
        Ok(file)
    }

    fn get_character(&self) -> Result<&String> {
        let Some(character) = self.character.as_ref() else {
            bail!("No character selected!")
        };
        Ok(character)
    }

    fn get_prompt(&self) -> Result<&Prompt> {
        let Some(prompt) = self.prompt.as_ref() else {
            bail!("No file loaded!")
        };
        Ok(prompt)
    }

    pub async fn reload_file(&mut self) -> Result<()> {
        let prompt: Prompt = parse_from_file(self.get_file()?)?;

        if Some(&prompt.config.server) != self.prompt.as_ref().map(|prompt| &prompt.config.server) {
            println!("Initializing server...");
            self.servers = Some(Servers::from_config(&prompt.config.server).await?);
            println!("Done!");
        }

        self.history.set_prompt(prompt.prompt.clone());
        self.prompt = Some(prompt);
        Ok(())
    }

    pub async fn load_file(&mut self, file: impl AsRef<Path>) -> Result<()> {
        self.file = Some(file.as_ref().to_path_buf());
        self.reload_file().await
    }

    pub async fn set_character(&mut self, character: String) -> Result<()> {
        if !self
            .get_prompt()?
            .characters
            .iter()
            .any(|char| char.name == character)
        {
            bail!("Character \"{character}\" does not exist within the current prompt!");
        }

        self.character = Some(character);

        Ok(())
    }

    pub async fn generate(&mut self) -> Result<()> {
        self.reload_file().await?;

        let Some(file) = self.file.as_ref() else {
            bail!("No file loaded!")
        };
        let Some(prompt) = self.prompt.as_ref() else {
            bail!("No file loaded!")
        };
        let Some(character) = self.character.as_ref() else {
            bail!("No character selected!")
        };
        let Some(servers) = self.servers.as_mut() else {
            bail!("No servers initialized!")
        };

        let (gen, abort) = servers.generate_with_preview(
            prompt.get_server_prompt(character)?,
            std::time::Duration::from_millis(100),
        );

        self.history.set_prompt(prompt.prompt.clone());

        let mut gen = Box::pin(gen);

        let mut generation = tokio::select! {
            res = &mut gen => {
                res?
            }
            _ = tokio::signal::ctrl_c() => {
                abort.await?;
                gen.await?
            }
        };
        println!();

        generation = prompt.finalize_response(character, generation)?;
        insert_response_into_file(file, &generation)?;
        self.history.add_response(&generation);
        self.reload_file().await?;
        Ok(())
    }

    fn write_prompt_to_file(&self) -> Result<()> {
        overwrite_prompt_in_file(self.get_file()?, self.history.prompt())
    }

    pub async fn run_command(&mut self, command: &str) -> Result<bool> {
        match command.parse()? {
            Command::Help => bail!("Command not yet implemented!"),
            Command::Exit => return Ok(false),
            Command::Load(file) => self.load_file(file).await?,
            Command::Reload => self.reload_file().await?,
            Command::Char(char) => self.set_character(char.clone()).await?,
            Command::Gen => self.generate().await?,
            Command::Swipe => {
                self.history.undo();
                self.write_prompt_to_file()?;
                self.generate().await?;
            }
            Command::Undo => {
                self.history.undo();
                self.write_prompt_to_file()?;
            }
            Command::Redo => {
                self.history.redo();
                self.write_prompt_to_file()?;
            }
            Command::SwipeList => {
                let responses = self.history.responses();

                for (i, response) in responses.iter().enumerate() {
                    println!("{i}: {:?}", &response[..response.len().min(80)]);
                }
            },
            Command::SwipeIndex(i) => {
                self.history.with_response(i)?;
                self.write_prompt_to_file()?;
            },
        }

        Ok(true)
    }

    pub async fn run(&mut self) -> Result<()> {
        let config = rustyline::config::Builder::new()
            .auto_add_history(true)
            .build();

        let mut rl = rustyline::DefaultEditor::with_config(config)?;

        loop {
            let line = match rl.readline("> ") {
                Ok(l) => l,
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("Use 'exit' to exit.");
                    continue;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    break;
                }
                Err(e) => return Err(e.into()),
            };

            match self.run_command(&line).await {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => println!("{e}"),
            }
        }

        Ok(())
    }
}
