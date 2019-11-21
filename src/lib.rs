//! The Golang Templating Language for Rust.
//!
//! ## Example
//! ```rust
//! use gtmpl;
//!
//! let output = gtmpl::template("Finally! Some {{ . }} for Rust", "gtmpl");
//! assert_eq!(&output.unwrap(), "Finally! Some gtmpl for Rust");
//! ```
#[cfg(test)]
#[macro_use]
extern crate gtmpl_derive;
extern crate gtmpl_value;
extern crate itertools;
#[macro_use]
extern crate lazy_static;
mod exec;
#[doc(inlne)]
pub mod funcs;
mod lexer;
mod node;
mod parse;
mod print_verb;
mod printf;
mod template;
mod utils;

#[doc(inline)]
pub use crate::template::Template;

#[doc(inline)]
pub use crate::exec::Context;

#[doc(inline)]
pub use gtmpl_value::Func;

#[doc(inline)]
pub use gtmpl_value::from_value;

pub use gtmpl_value::Value;

/// Provides simple basic templating given just a template sting and context.
///
/// ## Example
/// ```rust
/// let output = gtmpl::template("Finally! Some {{ . }} for Rust", "gtmpl");
/// assert_eq!(&output.unwrap(), "Finally! Some gtmpl for Rust");
/// ```
pub fn template<T: Into<Value>>(template_str: &str, context: T) -> Result<String, String> {
    let mut tmpl = Template::default();
    tmpl.parse(template_str)?;
    tmpl.render(&Context::from(context)?)
}
