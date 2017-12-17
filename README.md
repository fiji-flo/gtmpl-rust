# gtmpl-rust &emsp; [![Build Status]][travis] [![Latest Version]][crates.io]
[Build Status]: https://travis-ci.org/fiji-flo/gtmpl-rust.svg?branch=master
[travis]: https://travis-ci.org/fiji-flo/gtmpl-rust
[Latest Version]: https://img.shields.io/crates/v/gtmpl.svg
[crates.io]: https://crates.io/crates/gtmpl


**The Golang Templating Language for Rust**

---

```toml
[dependencies]
gtmpl = "0.3.0"
```

* [gtmpl at crates.io](https://crates.io/crate/gtmpl)
* [gtmpl documentation](https://docs.rs/crate/gtmpl)
* [golang documentation](https://golang.org/pkg/text/template/)

## Current Limitations

This is work in progress. Currently the following features are not supported:

* complex numbers
* the following functions have not been implemented:
  * `html`, `js`
* `printf` is not yet fully stable, but should support all *sane* input

## Usage

Basic template rendering can be achieved in a single line.

```rust
extern crate gtmpl;
use gtmpl;

fn basic_template_rendering() {
    let output = gtmpl::template("Finally! Some {{ . }} for Rust", "gtmpl");
    assert_eq!(&output.unwrap(), "Finally! Some gtmpl for Rust");
}
```

For more examples please take a look at the
[gtmpl documentation](https://docs.rs/crate/gtmpl).

## Context

We use [gtmpl_value](https://github.com/fiji-flo/gtmpl_value)'s Value as internal
data type. [gtmpl_derive](https://github.com/fiji-flo/gtmpl_derive) provides a
handy `derive` marco to generate the `From` implmentation for `Value`.

See:

* [gtmpl_value at crates.io](https://crates.io/crate/gtmpl_value)
* [gtmpl_value documentation](https://docs.rs/crate/gtmpl_value)
* [gtmpl_derive at crates.io](https://crates.io/crate/gtmpl_derive)
* [gtmpl_derive documentation](https://docs.rs/crate/gtmpl_derive)

## Why do we need this?

The main motivation for this is to make it easier to write dev-ops tools in Rust
which feel native. Docker and Helm (Kubernetes) use golang templating and feels
more native if tooling around them uses the same.
