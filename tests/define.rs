extern crate gtmpl;
use gtmpl::{Context, Template};

#[test]
fn simple_define() {
    let mut template = Template::new("shiny_template");
    template
        .parse(
            r#"{{ define "tmpl"}} some {{ end -}} there is {{- template "tmpl" -}} template"#,
        )
        .unwrap();

    let context = Context::empty();

    let output = template.render(context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is some template".to_string());
}
