use super::*;
use anyhow::{bail, Result};
use std::str::FromStr;

use std::path::Path;

const CONFIG_TAG: &str = "<|CONFIG|>";
const ENDCONFIG_TAG: &str = "<|ENDCONFIG|>";

const PROMPT_TAG: &str = "<|PROMPT|>";

const CHAR_TAG: &str = "<|CHAR|>";
const ENDCHAR_TAG: &str = "<|ENDCHAR|>";

const CONTEXT_TAG: &str = "<|CONTEXT";
const ENDCONTEXT_TAG: &str = "<|ENDCONTEXT|>";
const END_TAG: &str = "|>";

impl FromStr for Prompt {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let config: Config = {
            let Some(i) = s.find(CONFIG_TAG) else {
                bail!("No config found!")
            };
            let s = &s[i + CONFIG_TAG.len()..];

            let Some(j) = s.find(ENDCONFIG_TAG) else {
                bail!("No config found!")
            };

            serde_yaml::from_str(&s[..j])?
        };

        let prompt = s
            .find(PROMPT_TAG)
            .map(|i| trim_newline_left_right(&s[i + PROMPT_TAG.len()..]).to_string())
            .unwrap_or_default();

        let mut characters = Vec::new();
        let mut view = s;

        while let Some(i) = view.find(CHAR_TAG) {
            view = &view[i + CHAR_TAG.len()..];

            let Some(i) = view.find(ENDCHAR_TAG) else {
                bail!("Unclosed char tag!")
            };
            let char = serde_yaml::from_str::<Character>(&view[..i])?;

            characters.push(char);

            view = &view[i + ENDCHAR_TAG.len()..];
        }

        let mut contexts = HashMap::new();
        view = s;

        while let Some(i) = view.find(CONTEXT_TAG) {
            view = &view[i + CONTEXT_TAG.len()..];

            let Some(i) = view.find(END_TAG) else {
                break;
            };

            let name = view[..i].trim();
            view = &view[i + END_TAG.len()..];

            let Some(i) = view.find(ENDCONTEXT_TAG) else {
                bail!("Unclosed context tag!")
            };

            let definition = trim_newline_left_right(&view[..i]);
            contexts.insert(
                name.to_string(),
                replace_char_user(definition, name, &config.user_name),
            );

            view = &view[i + ENDCONTEXT_TAG.len()..];
        }

        Ok(Prompt {
            config,
            characters,
            contexts,
            prompt,
        })
    }
}

pub fn parse_from_file<T: FromStr>(path: impl AsRef<Path>) -> Result<T>
where
    anyhow::Error: From<T::Err>,
{
    Ok(preprocess_file(path)?.parse()?)
}

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub fn insert_response_into_file(file: impl AsRef<Path>, response: &str) -> Result<()> {
    let mut output_file = OpenOptions::new().read(true).write(true).open(file)?;

    if output_file.seek(SeekFrom::End(0))? >= 2 {
        output_file.seek(SeekFrom::End(-2))?;

        let mut seek = 0i64;
        let mut buf = [0; 2];

        output_file.read_exact(&mut buf[..])?;
        if buf[(1 + seek) as usize] == b'\n' {
            seek -= 1;
        }
        if buf[(1 + seek) as usize] == b'\r' {
            seek -= 1;
        }

        output_file.seek(SeekFrom::End(seek))?;
    }

    write!(output_file, "{response}")?;

    Ok(())
}

pub fn overwrite_prompt_in_file(file: impl AsRef<Path>, prompt: &str) -> Result<()> {
    let mut contents = String::new();
    File::open(&file)?.read_to_string(&mut contents)?;

    let Some(i) = contents.rfind(PROMPT_TAG) else {
        bail!("Cannot write prompt to file: file does not contain \"<|PROMPT|>\" tag!")
    };

    contents.truncate(i + PROMPT_TAG.len());
    contents.push('\n');
    contents.push_str(prompt);

    write!(File::create(&file)?, "{contents}")?;

    Ok(())
}
