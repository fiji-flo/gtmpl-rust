# gtmpl-rust &emsp; [![Build Status]][travis] [![Latest Version]][crates.io]
[Build Status]: https://travis-ci.org/fiji-flo/gtmpl-rust.svg?branch=master
[travis]: https://travis-ci.org/fiji-flo/gtmpl-rust
[Latest Version]: https://img.shields.io/crates/v/gtmpl.svg
[crates.io]: https://crates.io/crates/gtmpl


**The Golang Templating Language for Rust**

---

```toml
[dependencies]
gtmpl = "0.1.2"
```

* [gtmpl at crates.io](https://crates.io/crate/gtmpl)
* [gtmpl documentation](https://docs.rs/crate/gtmpl)
* [golang documentation](https://golang.org/pkg/text/template/)

## Current Limitations

This is work in progress. Currently the following features are not supported:

* complex numbers
* comparing different number types
  * `eq 1 1.0` will be `false`
* the following functions have not been implemented:
  * `html`, `js`, `call` and `printf`

For now we use [serde_json](https://github.com/serde-rs/json)'s Value as internal
data type. However, this can not support passing functions to the context. In a
future release we will move to a custom data type that will be compatible with
serde_json.

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

## Why do we need this?

The main motivation for this is to make it easier to write dev-ops tools in Rust
which feel native. Docker and Helm (Kubernetes) use golang templating and feels
more native if tooling around them uses the same.
