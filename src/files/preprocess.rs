use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

/// Computes the path of file2 relative to the directory file1 is in.
fn join_filename(file1: impl AsRef<Path>, file2: impl AsRef<Path>) -> PathBuf {
    let mut file1 = file1.as_ref().to_path_buf();
    file1.pop();
    file1.push(file2);
    file1
}

pub(super) fn trim_newline_right(mut s: &str) -> &str {
    if let Some('\n') = s.chars().next_back() {
        s = &s[..s.len() - 1];
    }

    if let Some('\r') = s.chars().next_back() {
        s = &s[..s.len() - 1];
    }

    s
}

pub(super) fn trim_newline_left(mut s: &str) -> &str {
    if let Some('\r') = s.chars().next() {
        s = &s[1..];
    }

    if let Some('\n') = s.chars().next() {
        s = &s[1..];
    }

    s
}

pub(super) fn trim_newline_left_right(s: &str) -> &str {
    trim_newline_right(trim_newline_left(s))
}

fn extract_json_string(file: impl AsRef<Path>, pointer: &str) -> Result<String> {
    let mut contents = String::new();
    File::open(file)?.read_to_string(&mut contents)?;
    let value: serde_json::Value = contents.parse()?;

    value
        .pointer(pointer)
        .with_context(|| anyhow!("Could not find pointer {pointer:?}: {value}"))?
        .as_str()
        .with_context(|| anyhow!("Value at {pointer:?} is not a string: {value}"))
        .map(|s| s.to_string())
}

const INCLUDE_TAG: &str = "<|INCLUDE";
const INCLUDE_JSON_TAG: &str = "<|JSON";
const END_TAG: &str = "|>";

/// Reads a file and processes 'INCLUDE' and 'JSON' statements.
pub fn preprocess_file(file: impl AsRef<Path>) -> Result<String> {
    let mut contents = String::new();
    File::open(&file)?.read_to_string(&mut contents)?;

    let mut out = String::new();

    let mut tags: Vec<(usize, usize, &str)> = contents
        .match_indices(INCLUDE_TAG)
        .chain(contents.match_indices(INCLUDE_JSON_TAG))
        .filter_map(|(tag_start, s)| {
            contents[tag_start..]
                .find(END_TAG)
                .map(|tag_end| (tag_start, tag_end + tag_start, s))
        })
        .collect();

    tags.sort_unstable_by_key(|(i, _, _)| *i);

    for (i, (tag_start, tag_end, tag)) in tags.iter().copied().enumerate() {
        let prev_tag_end = tags
            .get(i.wrapping_sub(1))
            .map(|(_, end, _)| *end + END_TAG.len())
            .unwrap_or(0);
        if prev_tag_end > tag_start {
            bail!("Missing end tag for Include statement!")
        }
        out.push_str(&contents[prev_tag_end..tag_start]);

        let tag_args = &contents[tag_start + tag.len()..tag_end];

        let replacement = match tag {
            INCLUDE_TAG => preprocess_file(join_filename(&file, tag_args.trim()))?,
            INCLUDE_JSON_TAG => {
                let mut iter = tag_args.split_whitespace();
                let json_file = iter.next().context("Too few arguments for JSON tag!")?;
                let json_pointer = iter.next().context("Too few arguments for JSON tag!")?;
                extract_json_string(json_file, json_pointer)?
            }
            _ => unreachable!(),
        };

        out.push_str(&replacement);
    }

    out.push_str(
        &contents[tags
            .last()
            .map(|(_, end, _)| *end + END_TAG.len())
            .unwrap_or(0)..],
    );

    Ok(out)
}
