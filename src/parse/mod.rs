mod parse;
mod preprocess;

pub use parse::*;
pub use preprocess::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Character {
    name: String,
    prefix: String,
    suffix: String,
    definition: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptConfig {
    prompt: String,
    max_length: usize,
    max_context_length: usize,
    quiet: bool,
    rep_pen: f64,
    rep_pen_range: usize,
    rep_pen_slope: f64,
    sampler_full_determinism: bool,
    sampler_order: Vec<usize>,
    sampler_seed: u64,
    stop_sequence: Vec<String>,
    temperature: f64,
    tfs: f64,
    top_a: f64,
    top_k: usize,
    top_p: f64,
    typical: f64,
}

impl Default for PromptConfig {
    fn default() -> Self {
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
            sampler_seed: rand::random::<u64>() % (1 << 50),
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    model_file: String,
    context_size: usize,
    threads: usize,
    blasbatchsize: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            model_file: String::new(),
            context_size: 1024,
            threads: 1,
            blasbatchsize: 512,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    user_name: String,
    // i.e. "{{char}}:" or "\n{{Char}}" or "### Instruction:"
    stop_sequence: Vec<String>,
    trim_stop_sequence: bool,
    default_prefix: String,
    default_suffix: String,
    prompt: PromptConfig,
    server: ServerConfig,
}

#[derive(Default, Debug)]
pub struct Prompt {
    config: Config,
    characters: Vec<Character>,
    prompt: String,
}
