extern crate gtmpl;
use gtmpl::{Context, Template};

#[test]
fn simple_define() {
    let mut template = Template::default();
    template
        .parse(r#"{{ define "tmpl"}} some {{ end -}} there is {{- template "tmpl" -}} template"#)
        .unwrap();

    let context = Context::empty();

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is some template".to_string());
}

#[test]
fn multiple_defines() {
    let mut template = Template::default();
    template
        .parse(
            r#"{{ define "tmpl1"}} some {{ end -}} {{- define "tmpl2"}} some other {{ end -}}
            there is {{- template "tmpl2" -}} template"#,
        )
        .unwrap();

    let context = Context::empty();

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is some other template".to_string());
}

#[cfg(feature = "dynamic_template")]
#[test]
fn dynamic_template() {
    let mut template = Template::default();
    template
        .parse(
            r#"{{ define "tmpl1"}} some {{ end -}} {{- define "tmpl2"}} some other {{ end -}}
            there is {{- template (.) -}} template"#,
        )
        .unwrap();

    let context = Context::from("tmpl2").unwrap();

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is some other template".to_string());
}
