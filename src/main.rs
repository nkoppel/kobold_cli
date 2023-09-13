#![allow(dead_code)]


mod parse;

use parse::*;

fn main() {
    // println!("{}", serde_yaml::to_string(&Config::default()).unwrap());
    println!("{:#?}", prompt_from_file("test.txt"));
    // insert_response_into_file("test.txt", "\n### Response:\n2 + 2 = 4\n### Instruction:\n").unwrap();
}
