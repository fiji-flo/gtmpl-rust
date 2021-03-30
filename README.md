# gtmpl-rust – Golang Templates for Rust

[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/gtmpl.svg
[crates.io]: https://crates.io/crates/gtmpl

---

[gtmpl-rust] provides the [Golang text/template] engine for Rust. This enables
seamless integration of Rust application into the world of devops tools around
[kubernetes], [docker] and whatnot.

## Getting Started

Add the following dependency to your Cargo manifest…
```toml
[dependencies]
gtmpl = "0.7"
```

and look at the docs:
* [gtmpl at crates.io](https://crates.io/crates/gtmpl)
* [gtmpl documentation](https://docs.rs/crate/gtmpl)
* [golang documentation](https://golang.org/pkg/text/template/)


It's not perfect, yet. Help and feedback is more than welcome.

## Some Examples

Basic template:
```rust
use gtmpl;

fn main() {
    let output = gtmpl::template("Finally! Some {{ . }} for Rust", "gtmpl");
    assert_eq!(&output.unwrap(), "Finally! Some gtmpl for Rust");
}
```

Adding custom functions:
```rust
use gtmpl_value::Function;
use gtmpl::{FuncError, gtmpl_fn, template, Value};

fn main() {
    gtmpl_fn!(
    fn add(a: u64, b: u64) -> Result<u64, FuncError> {
        Ok(a + b)
    });
    let equal = template(r#"{{ call . 1 2 }}"#, Value::Function(Function { f: add }));
    assert_eq!(&equal.unwrap(), "3");
}
```

Passing a struct as context:
```rust
use gtmpl_derive::Gtmpl;

#[derive(Gtmpl)]
struct Foo {
    bar: u8
}

fn main() {
    let foo = Foo { bar: 42 };
    let output = gtmpl::template("The answer is: {{ .bar }}", foo);
    assert_eq!(&output.unwrap(), "The answer is: 42");
}
```

Invoking a *method* on a context:
```rust

use gtmpl_derive::Gtmpl;
use gtmpl::{Func, FuncError, Value};

fn plus_one(args: &[Value]) -> Result<Value, FuncError> {
    if let Value::Object(ref o) = &args[0] {
        if let Some(Value::Number(ref n)) = o.get("num") {
            if let Some(i) = n.as_i64() {
                return Ok((i +1).into())
            }
        }
    }
    Err(anyhow!("integer required, got: {:?}", args))
}

#[derive(Gtmpl)]
struct AddMe {
    num: u8,
    plus_one: Func
}

fn main() {
    let add_me = AddMe { num: 42, plus_one };
    let output = gtmpl::template("The answer is: {{ .plus_one }}", add_me);
    assert_eq!(&output.unwrap(), "The answer is: 43");
}
```

## Current Limitations

This is work in progress. Currently the following features are not supported:

* complex numbers
* the following functions have not been implemented:
  * `html`, `js`
* `printf` is not yet fully stable, but should support all *sane* input

## Enhancements

Even though it was never intended to extend the syntax of Golang text/template
there might be some convenient additions:

### Dynamic Template

Enable `gtmpl_dynamic_template` in your `Cargo.toml`:
```toml
[dependencies.gtmpl]
version = "0.7"
features = ["gtmpl_dynamic_template"]

```

Now you can have dynamic template names for the `template` action.

#### Example

```rust
use gtmpl::{Context, Template};

fn main() {
    let mut template = Template::default();
    template
        .parse(
            r#"
            {{- define "tmpl1"}} some {{ end -}}
            {{- define "tmpl2"}} some other {{ end -}}
            there is {{- template (.) -}} template
            "#,
        )
        .unwrap();

    let context = Context::from("tmpl2");

    let output = template.render(&context);
    assert_eq!(output.unwrap(), "there is some other template".to_string());
}
```

The following syntax is used:
```
{{template (pipeline)}}
	The template with the name evaluated from the pipeline (parenthesized) is
    executed with nil data.

{{template (pipeline) pipeline}}
	The template with the name evaluated from the first pipeline (parenthesized)
    is executed with dot set to the value of the second pipeline.
```

## Context

We use [gtmpl_value]'s Value as internal data type. [gtmpl_derive] provides a
handy `derive` macro to generate the `From` implementation for `Value`.

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

The main motivation for this is to make it easier to write devops tools in Rust
that feel native. [docker] and [helm] ([kubernetes]) use golang templates and
it feels more native if tooling around them uses the same.

[gtmpl-rust]: https://github.com/fiji-flo/gtmpl-rust
[Golang text/template]: https://golang.org/pkg/text/template/
[kubernetes]: https://kubernetes.io
[helm]: https://github.com/kubernetes/helm/blob/master/docs/chart_best_practices/templates.md
[docker]: https://docker.com
[gtmpl_value]: https://github.com/fiji-flo/gtmpl_value
[gtmpl_derive]: https://github.com/fiji-flo/gtmpl_derive
