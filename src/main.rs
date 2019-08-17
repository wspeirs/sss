extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::fs::File;
use std::io::Read;
use pest::Parser;

mod script;
mod expression;

use script::Script;
//use program::Term;


#[cfg(debug_assertions)]
const _GRAMMAR: &'static str = include_str!("grammar.pest"); // relative to this file

#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to src
struct SSSParser;


fn main() {
    let mut f = File::open("tests/simple.sss").unwrap_or_else(|e| panic!("Error opening file: {}", e));

    // read the entire file into memory
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap_or_else(|e| panic!("Error reading file: {}", e));

    // parse the file
    let pairs = SSSParser::parse(Rule::script, &contents).unwrap_or_else(|e| panic!("Error parsing: {}", e)).next().unwrap();

    Script::new(pairs);

}
