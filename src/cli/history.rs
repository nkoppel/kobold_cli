use std::collections::HashMap;
use std::mem;

#[derive(Clone, Debug, Default)]
pub struct History {
    undos: Vec<String>,
    redos: Vec<String>,
    prompt: String,
    responses: HashMap<String, Vec<String>>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prompt(&self) -> &String {
        &self.prompt
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

    pub fn add_response(&mut self, response: String) {
        let responses = self.responses.entry(self.prompt.clone()).or_default();

        if !responses.contains(&response) {
            responses.push(response);
        }
    }

    pub fn responses(&self) -> &[String] {
        self.responses
            .get(&self.prompt)
            .map(|v| &v[..])
            .unwrap_or(&[])
    }
}
