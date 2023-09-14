use std::fs::File;
use std::io::{Read, Result};
use std::path::{Path, PathBuf};

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

        if let Some('\r') = s.chars().next_back() {
            s = &s[..s.len() - 1];
        }
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

pub(super) fn trim_newline(s: &str) -> &str {
    trim_newline_left(trim_newline_right(s))
}

const INCLUDE_TAG: &str = "<|INCLUDE";
const END_TAG: &str = "|>";

/// Reads a file and processes 'INCLUDE' statements.
// TODO: add support for INCLUDE_CHARACTER
pub fn preprocess_file(file: impl AsRef<Path>) -> Result<String> {
    let mut contents = String::new();
    File::open(&file)?.read_to_string(&mut contents)?;

    let mut out = String::new();
    let mut view = trim_newline_right(&contents[..]);

    while let Some(i) = view.find(INCLUDE_TAG) {
        out.push_str(&view[..i]);
        view = view[i + INCLUDE_TAG.len()..].trim_start();

        let Some(i) = view.find(END_TAG) else { break };
        let file_name = &view[..i];

        view = &view[i + END_TAG.len()..];
        out.push_str(&preprocess_file(join_filename(&file, file_name))?);
    }

    out.push_str(view);

    Ok(out)
}
