use crate::rust::FormalityLang;
use formality_core::parse::{CoreParse, ParseResult, Parser, Scope};
use formality_core::term;

#[term]
#[customize(parse)]
#[derive(Default)]
pub enum MaybeWithEffects {
    #[default]
    #[grammar()]
    None,

    #[cast]
    With(WithEffects),
}

impl CoreParse<FormalityLang> for MaybeWithEffects {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::single_variant(scope, text, "MaybeWithEffects", |p| {
            p.each_nonterminal(|effects, p| p.ok(Self::With(effects)))
        })
    }
}

#[term(with $effect)]
pub struct WithEffects {
    pub effect: BuiltinEffect,
}

#[term]
pub enum BuiltinEffect {
    #[grammar(const)]
    Const,
}
