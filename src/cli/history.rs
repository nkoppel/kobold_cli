use std::mem;

use anyhow::{bail, Result};
use radix_trie::{Trie, TrieCommon};

#[derive(Clone, Debug, Default)]
pub struct History {
    undos: Vec<String>,
    redos: Vec<String>,
    prompt: String,
    responses: Trie<String, Vec<String>>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prompt(&self) -> &String {
        &self.prompt
    }

    pub fn clear(&mut self) {
        self.undos.clear();
        self.redos.clear();
        self.prompt.clear();
        self.responses = Trie::new();
    }

    pub fn undo(&mut self) {
        if let Some(undo) = self.undos.pop() {
            self.redos.push(mem::replace(&mut self.prompt, undo))
        }
    }

    pub fn redo(&mut self) {
        if let Some(redo) = self.redos.pop() {
            self.undos.push(mem::replace(&mut self.prompt, redo))
        }
    }

    pub fn set_prompt(&mut self, prompt: String) {
        if prompt != self.prompt {
            self.redos.clear();
            self.undos.push(mem::replace(&mut self.prompt, prompt));
        }
    }

    pub fn add_response(&mut self, response: &str) {
        self.responses.map_with_default(
            self.prompt.clone(),
            |v| v.push(response.to_string()),
            vec![response.to_string()],
        );
    }

    pub fn responses(&self) -> &[String] {
        self.responses
            .get_ancestor_value(&self.prompt)
            .map(|v| &v[..])
            .unwrap_or(&[])
    }

    pub fn with_response(&mut self, index: usize) -> Result<()> {
        let Some(node) = self.responses.get_ancestor(&self.prompt) else {
            bail!("Prompt has no responses!");
        };

        let Some(prompt) = node.key() else {
            bail!("Prompt has no responses!");
        };

        let Some(response) = node.value().and_then(|v| v.get(index)) else {
            bail!("No response {index} for prompt!");
        };

        self.prompt = format!("{prompt}{response}");

        Ok(())
    }
}
