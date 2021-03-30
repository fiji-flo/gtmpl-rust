use gtmpl::{Context, Template};
use gtmpl_derive::Gtmpl;

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
fn range_and_define() {
    let mut template = Template::default();
    template
        .parse(
            r#"{{ define "foo" }}{{ $ }}{{ end -}}
                  {{ range $x := . -}}{{ template "foo" . }}{{- end }}"#,
        )
        .unwrap();

    let context = Context::from(vec![1, 2]);

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "12".to_string());

    let mut template = Template::default();
    template
        .parse(
            r#"{{ define "foo" }}{{ . }}{{ end -}}
                  {{ range $x := . -}}{{ template "foo" . }}{{- end }}"#,
        )
        .unwrap();

    let context = Context::from(vec![1, 2]);

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "12".to_string());

    let mut template = Template::default();
    template
        .parse(
            r#"{{ define "foo" }}{{ $ }}{{ end -}}
                  {{ range $x := . -}}{{ template "foo" }}{{- end }}"#,
        )
        .unwrap();

    let context = Context::from(vec![1, 2]);

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "<no value><no value>".to_string());

    let mut template = Template::default();
    template
        .parse(
            r#"{{ define "foo" }}{{ . }}{{ end -}}
                  {{ range $x := . -}}{{ template "foo" }}{{- end }}"#,
        )
        .unwrap();

    let context = Context::from(vec![1, 2]);

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "<no value><no value>".to_string());
}

#[test]
fn simple_define_context() {
    let mut template = Template::default();
    template
        .parse(r#"{{ define "tmpl"}} {{.}} {{ end -}} there is {{- template "tmpl" -}} template"#)
        .unwrap();

    let context = Context::from("some");

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is <no value> template".to_string());

    let mut template = Template::default();
    template
        .parse(r#"{{ define "tmpl"}} some {{ end -}} there is {{- template "tmpl" . -}} template"#)
        .unwrap();

    let context = Context::from("some");

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is some template".to_string());
}

#[test]
fn other_define_context() {
    #[derive(Gtmpl)]
    struct Other {
        pub foo: String,
    }
    let mut template = Template::default();
    template
        .parse(r#"{{ define "tmpl"}} some {{ end -}} there is {{- template "tmpl" . -}} template"#)
        .unwrap();

    let context = Context::from(Other {
        foo: "some".to_owned(),
    });

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

#[cfg(feature = "gtmpl_dynamic_template")]
#[test]
fn dynamic_template() {
    let mut template = Template::default();
    template
        .parse(
            r#"
            {{- define "tmpl1"}} some {{ end -}}
            {{- define "tmpl2"}} some other {{ end -}}
            there is {{- template (.) -}} template"#,
        )
        .unwrap();

    let context = Context::from("tmpl2");

    let output = template.render(&context);
    assert!(output.is_ok());
    assert_eq!(output.unwrap(), "there is some other template".to_string());
}
