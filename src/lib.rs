//! The Golang Templating Language for Rust.
//!
//! ## Example
//! ```rust
//! use gtmpl;
//!
//! let output = gtmpl::template("Finally! Some {{ . }} for Rust", "gtmpl");
//! assert_eq!(&output.unwrap(), "Finally! Some gtmpl for Rust");
//! ```
#[macro_use]
extern crate lazy_static;
extern crate itertools;
#[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
#[allow(unused_imports)]
#[macro_use]
extern crate gtmpl_value;
#[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
#[allow(unused_imports)]
#[macro_use]
extern crate gtmpl_derive;
mod lexer;
mod node;
mod parse;
#[doc(inlne)]
pub mod funcs;
mod template;
mod exec;
mod utils;

#[doc(inline)]
pub use template::Template;

#[doc(inline)]
pub use exec::Context;

#[doc(inline)]
pub use gtmpl_value::Func;

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
