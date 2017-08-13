//! The Golang Templating Language for Rust
//!
//! ## Example
//! ```rust
//! use gtmpl::{Context, Template};
//!
//! let mut template = Template::new("shiny_template");
//! template.parse("Finally! Some {{ . }} for Rust").unwrap();
//!
//! let context = Context::from_str("gtmpl").unwrap();
//!
//! let output = template.render(context);
//! assert_eq!(output.unwrap(), "Finally! Some gtmpl for Rust".to_string());
//! ```
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
pub mod funcs;
pub mod template;
pub mod exec;
mod utils;

pub use template::Template;

pub use exec::Context;
