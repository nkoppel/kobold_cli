use crate::files::{ServerPrompt, ServerConfig};
use reqwest::{Client, Response};
use tokio::process::{Child, Command};

// ./koboldcpp.py --model models/$MODEL --host 127.0.0.1 --threads 4 --stream --skiplauncher --blasbatchsize 2048 --contextsize $CONTEXT --unbantokens

struct Server {
    child: Child,
    url: String,
    last_prompt: String,
}

pub struct Servers {
    client: Client,
    servers: Vec<Server>,
}

impl Server {
    fn from_config(config: ServerConfig, port: u16) -> std::io::Result<Server> {
        let child = Command::new(&config.executable_file)
            .args(["--stream", "--skiplauncher", "--unbantokens"])
            .args(["--model", &config.model_file])
            .args(["--host", "127.0.0.1"])
            .args(["--port", &format!("{port}")])
            .args(["--threads", &format!("{}", config.threads)])
            .args(["--blasbatchsize", &format!("{}", config.blas_batch_size)])
            .args(["--contextsize", &format!("{}", config.context_size)])
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .spawn()?;

        Ok(Server {
            child,
            url: format!("127.0.0.1:{port}"),
            last_prompt: String::new(),
        })
    }

    // TODO: consider using the llama tokenizer to get a better estimate of cost
    fn generation_cost(&self, prompt: &str) -> i64 {
        let shared_prefix_length = self
            .last_prompt
            .as_bytes()
            .iter()
            .zip(prompt.as_bytes())
            .position(|(p1, p2)| p1 != p2)
            .unwrap_or(self.last_prompt.len().min(prompt.len()));

        let erased = self.last_prompt.len() - shared_prefix_length;

        ((prompt.len() - shared_prefix_length) * 2 + erased) as i64
    }

    async fn generate(
        &mut self,
        client: &Client,
        prompt: ServerPrompt,
    ) -> Result<Response, reqwest::Error> {
        let out = client
            .post(&format!("http://{}/api/v1/generate", self.url))
            .body(serde_json::to_string(&prompt).unwrap())
            .header("Content-Type", "application/json")
            .send()
            .await;

        self.last_prompt = prompt.prompt;
        out
    }

    async fn check(&mut self, client: &Client) -> Result<String, reqwest::Error> {
        let json: serde_json::Value = client
            .post(&format!("http://{}/api/extra/generate/check", self.url))
            .header("Content-Length", "0")
            .send().await?
            .json().await?;

        Ok(json["results"][0]["text"].to_string())
    }

    async fn abort(&mut self, client: &Client) -> Result<(), reqwest::Error> {
        client
            .post(&format!("http://{}/api/extra/generate/abort", self.url))
            .header("Content-Length", "0")
            .send().await
            .map(|_| ())
    }
}

impl Servers {
    pub fn from_config(config: ServerConfig, first_port: u16) -> std::io::Result<Servers> {
        let mut servers = Vec::new();

        for port in first_port..first_port + config.instances as u16 {
            servers.push(Server::from_config(config.clone(), port)?);
        }

        Ok(Servers {
            client: Client::new(),
            servers,
        })
    }

    fn best_server_index(&self, prompt: &str) -> usize {
        self.servers
            .iter()
            .map(|server| server.generation_cost(prompt))
            .enumerate()
            .min_by_key(|(_, x)| *x)
            .unwrap()
            .0
    }
}
