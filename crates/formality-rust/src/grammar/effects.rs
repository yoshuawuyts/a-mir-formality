use std::fmt;

use crate::grammar::EffectAliasId;
use crate::rust::FormalityLang;
use formality_core::parse::{
    ActiveVariant, CoreParse, ParseError, ParseResult, Parser, Precedence, Scope,
};
use formality_core::term;

#[term(eff $id = $effects ;)]
pub struct EffectAlias {
    pub id: EffectName,
    pub effects: BuiltinEffectList,
}

#[term]
#[customize(parse)]
pub enum EffectName {
    #[cast]
    Builtin(BuiltinEffect),
    #[cast]
    Alias(EffectAliasId),
}

impl CoreParse<FormalityLang> for EffectName {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::multi_variant(scope, text, "EffectName", |parser| {
            parser.parse_variant_cast::<BuiltinEffect>(Precedence::default());
            if !parser.has_success() {
                parser.parse_variant_cast::<EffectAliasId>(Precedence::default());
            }
        })
    }
}

#[term]
#[customize(parse, debug)]
#[derive(Default)]
pub struct EffectList {
    elements: Vec<EffectName>,
}

impl EffectList {
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub(crate) fn contains_builtin(&self, effect: BuiltinEffect) -> bool {
        self.elements.contains(&EffectName::Builtin(effect))
    }

    pub(crate) fn alias_id(&self) -> Option<&EffectAliasId> {
        match self.elements.as_slice() {
            [EffectName::Alias(id)] => Some(id),
            _ => None,
        }
    }
}

impl CoreParse<FormalityLang> for EffectList {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::multi_variant(scope, text, "EffectList", |parser| {
            parser.parse_variant("built-in effects", Precedence::default(), |p| {
                p.each_nonterminal(|BuiltinEffectList { elements }, p| {
                    p.ok(Self {
                        elements: elements.into_iter().map(EffectName::Builtin).collect(),
                    })
                })
            });
            parser.parse_variant("effect alias", Precedence::default(), |p| {
                // Preserve errors from a malformed list that starts with a built-in.
                p.reject_nonterminal::<BuiltinEffect>()?;
                p.each_nonterminal(|id, p| {
                    p.ok(Self {
                        elements: vec![EffectName::Alias(id)],
                    })
                })
            })
        })
    }
}

impl fmt::Debug for EffectList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_list("EffectList", &self.elements, f)
    }
}

#[term]
#[customize(parse, debug)]
pub struct BuiltinEffectList {
    elements: Vec<BuiltinEffect>,
}

impl BuiltinEffectList {
    pub(crate) fn contains(&self, effect: &BuiltinEffect) -> bool {
        self.elements.contains(effect)
    }

    fn parse_rest<'t>(self, p: &mut ActiveVariant<'_, 't, FormalityLang>) -> ParseResult<'t, Self> {
        if p.expect_char('+').is_err() {
            return p.ok(self);
        }

        p.skip_whitespace();
        let text = p.text();
        p.each_nonterminal(|effect, p| {
            if self.contains(&effect) {
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

impl CoreParse<FormalityLang> for BuiltinEffectList {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::single_variant(scope, text, "BuiltinEffectList", |p| {
            p.each_nonterminal(|effect, p| {
                Self {
                    elements: vec![effect],
                }
                .parse_rest(p)
            })
        })
    }
}

impl fmt::Debug for BuiltinEffectList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_list("BuiltinEffectList", &self.elements, f)
    }
}

fn fmt_list(name: &str, elements: &[impl fmt::Debug], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if f.alternate() {
        return f.debug_struct(name).field("elements", &elements).finish();
    }

    if let Some((first, rest)) = elements.split_first() {
        write!(f, "{first:?}")?;
        for effect in rest {
            write!(f, " + {effect:?}")?;
        }
    }
    Ok(())
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
