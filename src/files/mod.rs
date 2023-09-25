mod parse;
mod preprocess;

pub use parse::*;
pub use preprocess::*;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Character {
    pub(crate) name: String,
    pub(crate) temporary_prefix: String,
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) stop_sequence: Vec<String>,
    pub(crate) definition: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerPrompt {
    pub(crate) prompt: String,
    pub(crate) max_length: usize,
    pub(crate) max_context_length: usize,
    pub(crate) quiet: bool,
    pub(crate) rep_pen: f64,
    pub(crate) rep_pen_range: usize,
    pub(crate) rep_pen_slope: f64,
    pub(crate) sampler_full_determinism: bool,
    pub(crate) sampler_order: Vec<usize>,
    pub(crate) sampler_seed: u64,
    pub(crate) stop_sequence: Vec<String>,
    pub(crate) temperature: f64,
    pub(crate) tfs: f64,
    pub(crate) top_a: f64,
    pub(crate) top_k: usize,
    pub(crate) top_p: f64,
    pub(crate) typical: f64,
}

impl Default for ServerPrompt {
    fn default() -> Self {
        // Defaults taken from Kobold Lite.
        Self {
            prompt: String::new(),
            max_length: 50,
            max_context_length: 1024,
            quiet: false,
            rep_pen: 1.1,
            rep_pen_range: 320,
            rep_pen_slope: 0.7,
            sampler_full_determinism: false,
            sampler_order: vec![6, 0, 1, 3, 4, 2, 5],
            sampler_seed: rand::random::<u64>(),
            stop_sequence: Vec::new(),
            temperature: 0.7,
            tfs: 1.0,
            top_a: 0.0,
            top_k: 0,
            top_p: 0.92,
            typical: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub(crate) executable_file: String,
    pub(crate) model_file: String,
    pub(crate) context_size: usize,
    pub(crate) threads: usize,
    pub(crate) blas_batch_size: isize,
    pub(crate) instances: usize,
    pub(crate) port: u16,
    pub(crate) custom_args: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            executable_file: String::new(),
            model_file: String::new(),
            context_size: 1024,
            threads: 1,
            blas_batch_size: 512,
            instances: 1,
            port: 5001,
            custom_args: Vec::new(),
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub(crate) user_name: String,
    pub(crate) prompt: ServerPrompt,
    pub(crate) server: ServerConfig,
}

#[derive(Clone, Default, Debug)]
pub struct Prompt {
    pub(crate) config: Config,
    pub(crate) characters: Vec<Character>,
    pub(crate) prompt: String,
}

// May need to make this more efficient in the future
fn replace_char_user(s: &str, char: &str, user: &str) -> String {
    s.replace("{{char}}", char).replace("{{user}}", user)
}

impl Prompt {
    // This can be made more efficient
    fn stop_sequences(&self, character: &str) -> Result<Vec<String>> {
        let mut names: Vec<&str> = self.characters.iter().map(|char| &char.name[..]).collect();
        names.push(&self.config.user_name);

        Ok(self
            .get_character(character)?
            .stop_sequence
            .iter()
            .flat_map(|stop| {
                names
                    .iter()
                    .map(|name| replace_char_user(stop, name, &self.config.user_name))
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect())
    }

    fn get_character(&self, character: &str) -> Result<&Character> {
        self.characters
            .iter()
            .find(|char| char.name == character)
            .ok_or_else(|| anyhow!("No character with name {character}!"))
    }

    pub fn get_server_prompt(&self, character: &str) -> Result<ServerPrompt> {
        let mut out = self.config.prompt.clone();

        out.stop_sequence = self.stop_sequences(character)?;

        let character = self.get_character(character)?;

        out.prompt = character.definition.clone();
        out.prompt.push_str(&self.prompt);
        out.prompt.push_str(&character.temporary_prefix);
        out.prompt.push_str(&character.prefix);

        out.prompt = replace_char_user(&out.prompt, &character.name, &self.config.user_name);

        Ok(out)
    }

    pub fn finalize_response(&self, character: &str, mut response: String) -> Result<String> {
        let stop_sequences = self.stop_sequences(character)?;

        for sequence in &stop_sequences {
            if response.strip_suffix(sequence).is_some() {
                response.truncate(response.len() - sequence.len());
                break;
            }
        }

        let character = self.get_character(character)?;

        response.insert_str(0, &character.prefix);
        response.push_str(&character.suffix);

        Ok(response)
    }
}
