#![allow(dead_code)]
#[macro_use]
extern crate lazy_static;
extern crate itertools;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
mod lexer;
mod node;
mod parse;
mod funcs;
mod template;
mod exec;
mod utils;
