use crate::rust::FormalityLang;
use formality_core::parse::{CoreParse, ParseResult, Parser, Scope};
use formality_core::term;

#[term($*elements)]
#[customize(parse)]
#[derive(Default)]
pub struct EffectList {
    elements: Vec<BuiltinEffect>,
}

impl EffectList {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl CoreParse<FormalityLang> for EffectList {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::single_variant(scope, text, "EffectList", |p| {
            p.each_nonterminal(|effect, p| {
                p.ok(Self {
                    elements: vec![effect],
                })
            })
        })
    }
}

#[term]
pub enum BuiltinEffect {
    #[grammar(const)]
    Const,
}
