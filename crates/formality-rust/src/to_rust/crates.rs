use crate::grammar::{AdtItem, Crate, CrateItem, Crates, Fallible};
use crate::to_rust::{
    context::Context, feature_gate, fns, structs_enums_and_adts, syntax, traits_and_impls,
    CORE_CRATE_NAME, MY_CORE_CRATE_NAME, MY_CORE_IMPORT,
};
use std::collections::HashMap;

pub fn build_crates(ctx: &mut Context, crates: &Crates) -> Fallible<HashMap<String, String>> {
    let contains_core = crates
        .crates
        .iter()
        .find(|c| *(c.id) == CORE_CRATE_NAME)
        .is_some();

    crates
        .crates
        .iter()
        .map(|krate| {
            let lowered = lower_crate(ctx, krate, contains_core)?;
            let name = if *(krate.id) == CORE_CRATE_NAME {
                MY_CORE_CRATE_NAME.to_string()
            } else {
                (*(krate.id)).clone()
            };
            Ok((name, lowered.to_string()))
        })
        .collect()
}

pub fn lower_crate(
    ctx: &mut Context,
    krate: &Crate,
    add_core_import: bool,
) -> Fallible<syntax::RustCrate> {
    let mut attrs = Vec::new();
    // Do not import the core crate in itself.
    let mut items = if add_core_import && *(krate.id) != CORE_CRATE_NAME {
        vec![syntax::Item::Import((*MY_CORE_IMPORT).clone())]
    } else {
        Vec::new()
    };

    let is_core = *(krate.id) == CORE_CRATE_NAME;

    for item in &krate.items {
        match item {
            // `Sized` is a language item: emitting our own would shadow rustc's `Sized`
            // via `use mycore::*;`, so bare `Sized` bounds must resolve to the real one.
            CrateItem::Trait(t) if is_core && t.id.is_sized() => {}
            CrateItem::FeatureGate(gate) => attrs.push(feature_gate::lower_feature_gate(gate)?),
            CrateItem::AdtItem(AdtItem::Struct(strukt)) => {
                items.push(syntax::Item::Struct(structs_enums_and_adts::lower_struct(
                    ctx, strukt,
                )?));
            }
            CrateItem::AdtItem(AdtItem::Enum(e)) => {
                items.push(syntax::Item::Enum(structs_enums_and_adts::lower_enum(
                    ctx, e,
                )?));
            }
            CrateItem::Trait(t) => {
                items.push(syntax::Item::Trait(traits_and_impls::lower_trait(ctx, t)?));
            }
            CrateItem::TraitImpl(trait_impl) => {
                items.push(syntax::Item::Impl(traits_and_impls::lower_trait_impl(
                    ctx, trait_impl,
                )?));
            }
            CrateItem::NegTraitImpl(neg_trait_impl) => {
                items.push(syntax::Item::NegImpl(
                    traits_and_impls::lower_neg_trait_impl(ctx, neg_trait_impl)?,
                ));
            }
            CrateItem::Fn(function) => {
                items.push(syntax::Item::Function(fns::lower_fn(
                    ctx,
                    function,
                    syntax::Visibility::Public,
                )?));
            }
            CrateItem::Test(_) => {
                todo!("lowering `test` crate items is not implemented yet")
            }
            CrateItem::EffectAlias(alias) => {
                anyhow::bail!(
                    "Rust lowering does not support effect alias declarations (`{:?}`)",
                    alias.id
                );
            }
        }
    }

    Ok(syntax::RustCrate { attrs, items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codegen::codegen_program, rust::term};

    #[test]
    fn with_effect_alias_inputs_are_not_lowered_or_generated() {
        for (input, unsupported) in [
            (
                "[crate Dep { eff reporting = const; }, crate App { fn main() {} }]",
                "effect alias declarations (`reporting`)",
            ),
            (
                "[crate App { fn main() with reporting {} }]",
                "`with reporting` on functions",
            ),
            (
                "[crate App { fn main() { with reporting {} } }]",
                "`with reporting` on blocks",
            ),
            (
                "[crate App { fn main() { return || with reporting {}; } }]",
                "closures",
            ),
        ] {
            let program = term(input);
            let rust_error = build_crates(&mut Context::default(), &program).unwrap_err();
            let minirust_error = codegen_program(&program).unwrap_err();
            assert_eq!(
                rust_error.to_string(),
                format!("Rust lowering does not support {unsupported}"),
                "{input}"
            );
            assert!(
                minirust_error
                    .to_string()
                    .contains(&format!("MiniRust codegen does not support {unsupported}")),
                "{input}\n{minirust_error}"
            );
        }
    }
}
