use std::collections::HashMap;

use parse::{parse, Parser, Tree};
use funcs::BUILTINS;
use node::TreeId;

use gtmpl_value::Func;

/// The main template structure.
#[derive(Default)]
pub struct Template {
    pub name: String,
    pub text: String,
    pub funcs: HashMap<String, Func>,
    pub tree_ids: HashMap<TreeId, String>,
    pub tree_set: HashMap<String, Tree>,
}

impl Template {
    /// Creates a new empty template with a given `name`.
    pub fn with_name<T: Into<String>>(name: T) -> Template {
        Template {
            name: name.into(),
            text: String::from(""),
            funcs: HashMap::default(),
            tree_ids: HashMap::default(),
            tree_set: HashMap::default(),
        }
    }

    /// Adds a single custom function to the template.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use gtmpl::{Context, Func, Value};
    ///
    /// fn hello_world(_args: &[Value]) -> Result<Value, String> {
    ///   Ok(Value::from("Hello World!"))
    /// }
    ///
    /// let mut tmpl = gtmpl::Template::default();
    /// tmpl.add_func("helloWorld", hello_world);
    /// tmpl.parse("{{ helloWorld }}").unwrap();
    /// let output = tmpl.render(&Context::empty());
    /// assert_eq!(&output.unwrap(), "Hello World!");
    /// ```
    pub fn add_func(&mut self, name: &str, func: Func) {
        self.funcs.insert(name.to_owned(), func);
    }

    /// Adds custom functions to the template.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    ///
    /// use gtmpl::{Context, Func, Value};
    ///
    /// fn hello_world(_args: &[Value]) -> Result<Value, String> {
    ///   Ok(Value::from("Hello World!"))
    /// }
    ///
    /// let funcs = vec![("helloWorld", hello_world as Func)];
    /// let mut tmpl = gtmpl::Template::default();
    /// tmpl.add_funcs(&funcs);
    /// tmpl.parse("{{ helloWorld }}").unwrap();
    /// let output = tmpl.render(&Context::empty());
    /// assert_eq!(&output.unwrap(), "Hello World!");
    /// ```
    pub fn add_funcs<T: Into<String> + Clone>(&mut self, funcs: &[(T, Func)]) {
        self.funcs
            .extend(funcs.iter().cloned().map(|(k, v)| (k.into(), v)));
    }

    /// Parse the given `text` as template body.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut tmpl = gtmpl::Template::default();
    /// tmpl.parse("Hello World!").unwrap();
    /// ```
    pub fn parse<T: Into<String>>(&mut self, text: T) -> Result<(), String> {
        let mut funcs = HashMap::new();
        funcs.extend(BUILTINS.iter().map(|&(k, v)| (k.to_owned(), v)));
        funcs.extend(self.funcs.clone());
        let parser = parse(self.name.clone(), text.into(), funcs)?;
        match parser {
            Parser {
                funcs,
                tree_ids,
                tree_set,
                ..
            } => {
                self.funcs = funcs;
                self.tree_set = tree_set;
                self.tree_ids = tree_ids;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_mocked {
    use super::*;

    #[test]
    fn test_parse() {
        let mut t = Template::with_name("foo");
        assert!(t.parse(r#"{{ if eq "bar" "bar" }} 2000 {{ end }}"#).is_ok());
        assert!(t.tree_set.contains_key("foo"));
        assert!(t.tree_ids.contains_key(&1usize));
    }
}
