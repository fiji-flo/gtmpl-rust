pub struct Template {
    name: String,
}

impl Template {
    pub fn new(name: &str) -> Template {
        Template { name: name.to_owned() }
    }

    pub fn parse(&mut self, text: String) {}
}
