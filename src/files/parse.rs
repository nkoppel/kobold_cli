use super::*;
use anyhow::{bail, Result};

use std::path::Path;

const CONFIG_TAG: &str = "<|CONFIG|>";
const ENDCONFIG_TAG: &str = "<|ENDCONFIG|>";

const PROMPT_TAG: &str = "<|PROMPT|>";
const ENDPROMPT_TAG: &str = "<|ENDPROMPT|>";

const CHAR_TAG: &str = "<|CHAR|>";
const CHAR_DEFINITION_TAG: &str = "<|CHAR_DEFINITION|>";
const ENDCHAR_TAG: &str = "<|ENDCHAR|>";

const END_TAG: &str = "|>";

pub fn parse_prompt(s: &str) -> Result<Prompt> {
    let config = {
        let Some(i) = s.find(CONFIG_TAG) else {
            bail!("No config found!")
        };
        let s = &s[i + CONFIG_TAG.len()..];

        let Some(j) = s.find(ENDCONFIG_TAG) else {
            bail!("No config found!")
        };

        serde_yaml::from_str(&s[..j])?
    };

    let mut prompt = String::new();
    let mut view = s;

    while let Some(i) = view.find(PROMPT_TAG) {
        view = &view[i + PROMPT_TAG.len()..];

        let Some(i) = view.find(ENDPROMPT_TAG) else {
            break;
        };

        prompt.push_str(trim_newline_left(&view[..i]));
        view = &view[i + ENDPROMPT_TAG.len()..];
    }

    prompt = trim_newline_right(&prompt).to_string();

    let mut characters = Vec::new();
    let mut view = s;

    while let Some(i) = view.find(CHAR_TAG) {
        view = &view[i + CHAR_TAG.len()..];

        let Some(i) = view.find(CHAR_DEFINITION_TAG) else {
            break;
        };
        let mut char = serde_yaml::from_str::<Character>(&view[..i])?;

        view = &view[i + CHAR_DEFINITION_TAG.len()..];
        let Some(i) = view.find(ENDCHAR_TAG) else {
            break;
        };
        char.definition = trim_newline_left(&view[..i]).to_string();

        characters.push(char);
    }

    Ok(Prompt {
        config,
        characters,
        prompt,
    })
}

pub fn prompt_from_file(path: impl AsRef<Path>) -> Result<Prompt> {
    parse_prompt(&preprocess_file(path)?)
}

use std::fs::File;
use std::io::{Read, Write};

pub fn insert_response_into_file(file: impl AsRef<Path>, response: &str) -> Result<()> {
    let mut contents = String::new();
    File::open(&file)?.read_to_string(&mut contents)?;

    let Some(mut i) = contents.rfind(ENDPROMPT_TAG) else {
        bail!("No ENDPROMPT in file!")
    };

    if let Some(b'\n') = contents.as_bytes().get(i.wrapping_sub(1)) {
        i -= 1;
        if let Some(b'\r') = contents.as_bytes().get(i.wrapping_sub(1)) {
            i -= 1;
        }
    }

    let mut output_file = File::create(&file)?;

    output_file.write_all(contents[..i].as_bytes())?;
    output_file.write_all(response.as_bytes())?;
    output_file.write_all(contents[i..].as_bytes())?;

    Ok(())
}
