use std::fmt;

use failure;

use liquid;

// TODO(epage): Look into making template system pluggable
// - Leverage traits
// - Possibly get liquid to also work with serializables like Tera(?)
// But should we?  Would it be better to have consistency in syntax and functionality?
// Either way, might be better to switch to another template engine if it looks like its getting
// traction within Rust community (like whatever is used for cargo templates) and to one that will
// be 1.0 sooner.
/// String-templating engine for staging fields.
pub struct TemplateEngine {
    parser: liquid::Parser,
    globals: liquid::Object,
}

impl TemplateEngine {
    /// Create a new string-template engine, initialized with `global` variables.
    pub fn new(globals: liquid::Object) -> Result<Self, failure::Error> {
        // TODO(eage): Better customize liquid
        // - Add raw block
        // - Remove irrelevant filters (like HTML ones)
        // - Add path manipulation filters
        let parser = liquid::ParserBuilder::new().liquid_filters().build();
        Ok(Self { parser, globals })
    }

    /// Evaluate `template`.
    pub fn render(&self, template: &str) -> Result<String, failure::Error> {
        // TODO(epage): get liquid to be compatible with failure::Fail
        let template = self.parser.parse(template)?;
        let content = template.render(&self.globals)?;
        Ok(content)
    }
}

impl fmt::Debug for TemplateEngine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TemplateEngine")
            .field("parser", &"?")
            .field("globals", &self.globals)
            .finish()
    }
}

/// Translate user-facing value to a staging value.
pub trait TemplateRender {
    /// Data type the template generates.
    type Rendered;

    /// Evaluate into `Rendered` using `engine`.
    fn format(&self, engine: &TemplateEngine) -> Result<Self::Rendered, failure::Error>;
}

/// Stager field that is a single template string.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Template(String);

impl Template {
    /// Treat `s` as a template string.
    pub fn new<S>(s: S) -> Self
    where
        S: Into<String>,
    {
        Self { 0: s.into() }
    }
}

impl TemplateRender for Template {
    type Rendered = String;

    fn format(&self, engine: &TemplateEngine) -> Result<String, failure::Error> {
        engine.render(&self.0)
    }
}

/// Stager field that is logically a sequence of templates but can be shortened to a single value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// Short-cut for a sequence of template-strings.
    One(T),
    /// Template-strings.
    Many(Vec<T>),
}

impl<T> TemplateRender for OneOrMany<T>
where
    T: TemplateRender,
{
    type Rendered = Vec<T::Rendered>;

    fn format(&self, engine: &TemplateEngine) -> Result<Self::Rendered, failure::Error> {
        match *self {
            OneOrMany::One(ref v) => {
                let u = v.format(engine)?;
                Ok(vec![u])
            }
            OneOrMany::Many(ref v) => {
                let u: Result<Vec<_>, _> = v.iter().map(|a| a.format(engine)).collect();
                u
            }
        }
    }
}
