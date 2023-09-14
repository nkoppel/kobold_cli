#![allow(dead_code)]

mod files;
mod server;

use files::*;

fn main() {
    // println!("{}", serde_yaml::to_string(&Config::default()).unwrap());
    let prompt = prompt_from_file("test.txt").unwrap();
    let server_prompt = prompt.get_server_prompt("MathInstruct");

    println!("{}", serde_json::to_string(&server_prompt).unwrap());
    // insert_response_into_file("test.txt", "\n### Response:\n2 + 2 = 4\n### Instruction:\n").unwrap();
}
