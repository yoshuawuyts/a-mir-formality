use std::fmt;

use crate::rust::FormalityLang;
use formality_core::parse::{ActiveVariant, CoreParse, ParseError, ParseResult, Parser, Scope};
use formality_core::term;

#[term]
#[customize(parse, debug)]
#[derive(Default)]
pub struct EffectList {
    elements: Vec<BuiltinEffect>,
}

impl EffectList {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    fn parse_rest<'t>(self, p: &mut ActiveVariant<'_, 't, FormalityLang>) -> ParseResult<'t, Self> {
        if p.expect_char('+').is_err() {
            return p.ok(self);
        }

        p.skip_whitespace();
        let text = p.text();
        p.each_nonterminal(|effect, p| {
            if self.elements.contains(&effect) {
                return Err(ParseError::at(
                    text,
                    format!("duplicate built-in effect `{effect:?}`"),
                ));
            }

            let mut list = self.clone();
            list.elements.push(effect);
            list.parse_rest(p)
        })
    }
}

impl CoreParse<FormalityLang> for EffectList {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::single_variant(scope, text, "EffectList", |p| {
            p.each_nonterminal(|effect, p| {
                Self {
                    elements: vec![effect],
                }
                .parse_rest(p)
            })
        })
    }
}

impl fmt::Debug for EffectList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return f
                .debug_struct("EffectList")
                .field("elements", &self.elements)
                .finish();
        }

        if let Some((first, rest)) = self.elements.split_first() {
            write!(f, "{first:?}")?;
            for effect in rest {
                write!(f, " + {effect:?}")?;
            }
        }
        Ok(())
    }
}

#[term]
pub enum BuiltinEffect {
    #[grammar(const)]
    Const,

    #[grammar(async)]
    Async,

    #[grammar(unwind)]
    Unwind,
}
