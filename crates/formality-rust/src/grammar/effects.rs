use std::fmt;

use crate::rust::FormalityLang;
use formality_core::parse::{
    ActiveVariant, CoreParse, ParseError, ParseResult, ParseSuccessType, Parser, Precedence, Scope,
};
use formality_core::{id, term};

id!(EffectAliasId, regex = r"[a-z_][a-z0-9_]*\b");

#[term(eff $name = $effects ;)]
pub struct EffectAlias {
    pub name: EffectName,
    pub effects: BuiltinEffectList,
}

#[term]
#[customize(parse, debug)]
pub enum EffectName {
    Builtin(BuiltinEffect),
    Alias(EffectAliasId),
}

impl CoreParse<FormalityLang> for EffectName {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::multi_variant(scope, text, "EffectName", |parser| {
            parser.parse_variant("builtin", Precedence::default(), |p| {
                p.each_nonterminal(|effect, p| p.ok(Self::Builtin(effect)))
            });
            parser.parse_variant("alias", Precedence::default(), |p| {
                p.reject_nonterminal::<BuiltinEffect>()?;
                p.each_nonterminal(|id, p| p.ok(Self::Alias(id)))
            });
        })
    }
}

impl fmt::Debug for EffectName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(effect) => effect.fmt(f),
            Self::Alias(id) => id.fmt(f),
        }
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

    pub(crate) fn alias_name(&self) -> Option<&EffectName> {
        match self.elements.as_slice() {
            [name @ EffectName::Alias(_)] => Some(name),
            _ => None,
        }
    }
}

impl CoreParse<FormalityLang> for EffectList {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::single_variant(scope, text, "EffectList", |p| {
            p.each_nonterminal(|name, p| match name {
                EffectName::Builtin(effect) => {
                    parse_builtin_tail(vec![effect], p, &|elements| Self {
                        elements: elements.into_iter().map(EffectName::Builtin).collect(),
                    })
                }
                EffectName::Alias(_) => p.ok(Self {
                    elements: vec![name],
                }),
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

        fmt_list(&self.elements, f)
    }
}

#[term]
#[customize(parse, debug)]
pub struct BuiltinEffectList {
    elements: Vec<BuiltinEffect>,
}

impl CoreParse<FormalityLang> for BuiltinEffectList {
    fn parse<'t>(scope: &Scope<FormalityLang>, text: &'t str) -> ParseResult<'t, Self> {
        Parser::single_variant(scope, text, "BuiltinEffectList", |p| {
            p.each_nonterminal(|effect, p| {
                parse_builtin_tail(vec![effect], p, &|elements| Self { elements })
            })
        })
    }
}

impl fmt::Debug for BuiltinEffectList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return f
                .debug_struct("BuiltinEffectList")
                .field("elements", &self.elements)
                .finish();
        }

        fmt_list(&self.elements, f)
    }
}

fn parse_builtin_tail<'t, R: ParseSuccessType>(
    elements: Vec<BuiltinEffect>,
    p: &mut ActiveVariant<'_, 't, FormalityLang>,
    build: &impl Fn(Vec<BuiltinEffect>) -> R,
) -> ParseResult<'t, R> {
    if p.expect_char('+').is_err() {
        return p.ok(build(elements));
    }

    p.skip_whitespace();
    let text = p.text();
    p.each_nonterminal(|effect, p| {
        if elements.contains(&effect) {
            return Err(ParseError::at(
                text,
                format!("duplicate built-in effect `{effect:?}`"),
            ));
        }

        let mut elements = elements.clone();
        elements.push(effect);
        parse_builtin_tail(elements, p, build)
    })
}

fn fmt_list(elements: &[impl fmt::Debug], f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
