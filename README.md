# gtmpl-rust &emsp; [![Build Status]][travis] [![Latest Version]][crates.io]
[Build Status]: https://travis-ci.org/fiji-flo/gtmpl-rust.svg?branch=master
[travis]: https://travis-ci.org/fiji-flo/gtmpl-rust
[Latest Version]: https://img.shields.io/crates/v/gtmpl.svg
[crates.io]: https://crates.io/crates/gtmpl


**The Golang Templating Language for Rust**

---

[gtmpl-rust] provides the [Golang text/template] system for Rust. This enables
seamless intergration of Rust application into the world of devops tools around
[kubernetes], [docker] and whatnot.

## Getting Started

Add the following dependency to your Cargo manifest…
```toml
[dependencies]
gtmpl = "0.3"
```

and look at the docs:
* [gtmpl at crates.io](https://crates.io/crate/gtmpl)
* [gtmpl documentation](https://docs.rs/crate/gtmpl)
* [golang documentation](https://golang.org/pkg/text/template/)


It's not perfect, yet. Help and feedback is more than welcome.

## Some Examples

Baisc template:
```rust
extern crate gtmpl;
use gtmpl;

fn main() {
    let output = gtmpl::template("Finally! Some {{ . }} for Rust", "gtmpl");
    assert_eq!(&output.unwrap(), "Finally! Some gtmpl for Rust");
}
```

Adding custom functions:
```rust
#[macro_use]
extern crate gtmpl;
extern crate gtmpl_value;
use gtmpl_value::Function;
use gtmpl::{template, Value};

fn main() {
    gtmpl_fn!(
    fn add(a: u64, b: u64) -> Result<u64, String> {
        Ok(a + b)
    });
    let equal = template(r#"{{ call . 1 2 }}"#, Value::Function(Function { f: add }));
    assert_eq!(&equal.unwrap(), "3");
}
```

## Current Limitations

This is work in progress. Currently the following features are not supported:

* complex numbers
* the following functions have not been implemented:
  * `html`, `js`
* `printf` is not yet fully stable, but should support all *sane* input

## Context

We use [gtmpl_value]'s Value as internal data type. [gtmpl_derive] provides a
handy `derive` marco to generate the `From` implmentation for `Value`.

See:

* [gtmpl_value at crates.io](https://crates.io/crate/gtmpl_value)
* [gtmpl_value documentation](https://docs.rs/crate/gtmpl_value)
* [gtmpl_derive at crates.io](https://crates.io/crate/gtmpl_derive)
* [gtmpl_derive documentation](https://docs.rs/crate/gtmpl_derive)

## Why do we need this?

Why? Dear god, why? I can already imagine the question coming up why anyone would
ever do this. I wasn't a big fan of Golang templates when i first had to write
some custom formatting strings for **docker**. Learning a new template language
usually isn't something one is looking forward to. Most people avoid it
completely. However, it's really useful for automation if you're looking for
something more lightweight than a full blown DSL.

The main motivation for this is to make it easier to write dev-ops tools in Rust
that feel native. [docker] and [helm] ([kubernetes]) use golang templating and
it feels more native if tooling around them uses the same.

[gtmpl-rust]: https://github.com/fiji-flo/gtmpl-rust
[kubernetes]: https://kubernetes.io
[helm]: https://github.com/kubernetes/helm/blob/master/docs/chart_best_practices/templates.md
[docker]: https://docker.com
[gtmpl_value]: https://github.com/fiji-flo/gtmpl_value
[gtmpl_derive]: https://github.com/fiji-flo/gtmpl_derive
