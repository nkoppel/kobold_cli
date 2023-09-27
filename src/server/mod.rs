use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::files::{ServerConfig, ServerPrompt};
use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::Value;
use tokio::process::{Child, Command};

use std::os::unix::process::CommandExt;

pub struct Servers {
    client: Client,
    children: Vec<Child>,
    urls: Vec<String>,
    last_prompts: Vec<String>,
    current_server: usize,
}

fn spawn_server(config: ServerConfig, port: u16) -> Result<Child> {
    let mut cmd = std::process::Command::new(&config.executable_file);

    cmd.args(["--stream", "--skiplauncher", "--unbantokens"])
        .args(["--model", &config.model_file])
        .args(["--host", "127.0.0.1"])
        .args(["--port", &format!("{port}")])
        .args(["--threads", &format!("{}", config.threads)])
        .args(["--blasbatchsize", &format!("{}", config.blas_batch_size)])
        .args(["--contextsize", &format!("{}", config.context_size)])
        .args(&config.custom_args)
        .current_dir(
            Path::new(&config.executable_file)
                .parent()
                .expect("Executable file has no parent directory!"),
        )
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());

    cmd.process_group(0);

    let mut cmd: Command = cmd.into();

    cmd.kill_on_drop(true);
    unsafe {
        cmd.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGHUP);
            Ok(())
        });
    }

    Ok(cmd.spawn()?)
}

// TODO: consider using token count instead of character count.
// TODO: consider a time decay for erased character cost.
fn generation_cost(last_prompt: &str, new_prompt: &str) -> f64 {
    let shared_prefix_length = last_prompt
        .as_bytes()
        .iter()
        .zip(new_prompt.as_bytes())
        .position(|(p1, p2)| p1 != p2)
        .unwrap_or(last_prompt.len().min(new_prompt.len()));

    let erased = last_prompt.len() - shared_prefix_length;

    (new_prompt.len() - shared_prefix_length) as f64 + erased as f64 / 2.
}

impl Servers {
    pub async fn from_config(config: &ServerConfig) -> Result<Servers> {
        let mut children = Vec::new();
        let mut urls = Vec::new();
        let mut last_prompts = Vec::new();
        let client = Client::new();
        let ports = config.port..config.port + config.instances as u16;

        for port in ports {
            children.push(spawn_server(config.clone(), port)?);
            urls.push(format!("127.0.0.1:{port}"));
            last_prompts.push(String::new());
        }

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let handles = urls
                .iter()
                .map(|url| tokio::spawn(is_online(client.clone(), url.clone())))
                .collect::<Vec<_>>();

            let mut out = true;

            for handle in handles {
                out &= handle.await??;
            }

            if out {
                break;
            }
        }

        Ok(Servers {
            client,
            children,
            urls,
            last_prompts,
            current_server: 0,
        })
    }

    fn best_server(&self, prompt: &str) -> usize {
        self.last_prompts
            .iter()
            .map(|last_prompt| generation_cost(last_prompt, prompt))
            .enumerate()
            .min_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap())
            .expect("No server instances!")
            .0
    }
}

async fn is_online(client: Client, url: String) -> Result<bool> {
    let res = client
        .post(&format!("http://{url}/api/v1/info/version"))
        .header("Content-Length", "0")
        .send()
        .await;

    match res {
        Ok(_) => Ok(true),
        Err(e) if e.is_connect() => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn check_request(client: &Client, url: &str) -> Result<String> {
    let json: serde_json::Value = client
        .post(&format!("http://{url}/api/extra/generate/check"))
        .header("Content-Length", "0")
        .send()
        .await?
        .json()
        .await?;

    let Some(s) = json.pointer("/results/0/text").and_then(Value::as_str) else {
        bail!("Received invalid json: {json:?}");
    };

    Ok(s.to_string())
}

async fn generate_request(client: Client, url: String, prompt: ServerPrompt) -> Result<String> {
    let json: serde_json::Value = client
        .post(format!("http://{url}/api/v1/generate"))
        .json(&prompt)
        .send()
        .await?
        .json()
        .await?;

    let Some(s) = json.pointer("/results/0/text").and_then(Value::as_str) else {
        bail!("Received invalid json: {json:?}");
    };

    Ok(s.to_string())
}

async fn abort_request(client: Client, url: String) -> Result<()> {
    client
        .post(&format!("http://{url}/api/extra/abort"))
        .header("Content-Length", "0")
        .send()
        .await?;

    Ok(())
}

async fn check_actor(
    client: Client,
    url: String,
    mut abort: tokio::sync::mpsc::Receiver<()>,
    interval: Duration,
) -> Result<()> {
    let mut check_len = 0;

    loop {
        tokio::time::sleep(interval).await;

        if abort.try_recv().is_ok() {
            break;
        }

        let check = check_request(&client, &url).await?;

        if check_len > check.len() {
            break;
        }
        print!("{}", &check[check_len..]);
        std::io::stdout().flush()?;
        check_len = check.len();
    }

    Ok(())
}

use std::future::Future;

/// Ensure that no progress is made on a future until it is awaited
async fn await_before<Func, F, T>(f: Func) -> T
where
    Func: FnOnce() -> F,
    F: Future<Output = T>,
{
    std::future::ready(()).await;
    f().await
}

impl Servers {
    pub fn generate(
        &mut self,
        prompt: ServerPrompt,
    ) -> (
        impl Future<Output = Result<String>>,
        impl Future<Output = Result<()>>,
    ) {
        let best_server = self.best_server(&prompt.prompt);
        let client = self.client.clone();
        let url = self.urls[best_server].clone();
        self.last_prompts[best_server] = prompt.prompt.clone();

        (
            generate_request(client.clone(), url.clone(), prompt),
            await_before(move || abort_request(client, url)),
        )
    }

    pub fn generate_with_preview(
        &mut self,
        prompt: ServerPrompt,
        check_interval: Duration,
    ) -> (
        impl Future<Output = Result<String>>,
        impl Future<Output = Result<()>>,
    ) {
        use tokio::sync::mpsc::*;

        async fn abort(client: Client, url: String, stop_check: Sender<()>) -> Result<()> {
            let _ = stop_check.send(()).await;
            abort_request(client, url).await
        }

        async fn generate(
            client: Client,
            url: String,
            prompt: ServerPrompt,
            stop_check: Sender<()>,
        ) -> Result<String> {
            let out = generate_request(client, url, prompt).await;
            let _ = stop_check.send(()).await;
            out
        }

        let best_server = self.best_server(&prompt.prompt);
        let client = self.client.clone();
        let url = self.urls[best_server].clone();
        let (send, recv) = channel(1);
        self.last_prompts[best_server] = prompt.prompt.clone();

        tokio::spawn(check_actor(
            client.clone(),
            url.clone(),
            recv,
            check_interval,
        ));

        print!("{}", prompt.prompt);
        std::io::stdout().flush().unwrap();

        (
            generate(client.clone(), url.clone(), prompt, send.clone()),
            await_before(move || abort(client, url, send)),
        )
    }
}
