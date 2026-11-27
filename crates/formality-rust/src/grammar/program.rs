use crate::grammar::{
    AdtId, AdtItem, Crate, CrateId, CrateItem, EffectAlias, EffectAliasId, Fallible, Fn, Struct,
    Trait, TraitId, ValueId,
};
use formality_core::term;

#[term($crates)]
pub struct Crates {
    /// List of all crates.
    /// The last crate in the list is the current crate.
    pub crates: Vec<Crate>,
}

impl Crates {
    pub fn len(&self) -> usize {
        self.crates.len()
    }

    /// Inclusive prefix.
    pub fn prefix(&self, tail: usize) -> Crates {
        assert!(
            tail < self.crates.len(),
            "Crates::prefix: tail {tail} out of range (len is {})",
            self.crates.len()
        );
        Crates {
            crates: self.crates[..=tail].to_vec(),
        }
    }

    pub fn items_from_all_crates(&self) -> impl Iterator<Item = &CrateItem> {
        self.crates.iter().flat_map(|c| &c.items)
    }

    fn items_with_crate_index(&self) -> impl Iterator<Item = (usize, &CrateItem)> {
        self.crates
            .iter()
            .enumerate()
            .flat_map(|(i, c)| c.items.iter().map(move |item| (i, item)))
    }

    pub fn fn_named(&self, fn_id: &ValueId) -> Fallible<&Fn> {
        self.fn_named_with_crate_index(fn_id).map(|(_, f)| f)
    }

    pub(crate) fn fn_named_with_crate_index(&self, fn_id: &ValueId) -> Fallible<(usize, &Fn)> {
        let mut fns = self
            .items_with_crate_index()
            .filter_map(|(i, item)| match item {
                CrateItem::Fn(f) if f.id == *fn_id => Some((i, f)),
                _ => None,
            });
        let Some(decl) = fns.next() else {
            anyhow::bail!("no fn named `{fn_id:?}`");
        };
        if fns.next().is_some() {
            anyhow::bail!("multiple fn named `{fn_id:?}`");
        }
        Ok(decl)
    }

    pub fn trait_named(&self, trait_id: &TraitId) -> Fallible<&Trait> {
        self.trait_named_with_crate_index(trait_id).map(|(_, t)| t)
    }

    pub(crate) fn trait_named_with_crate_index(
        &self,
        trait_id: &TraitId,
    ) -> Fallible<(usize, &Trait)> {
        let mut traits = self
            .items_with_crate_index()
            .filter_map(|(i, item)| match item {
                CrateItem::Trait(t) if t.id == *trait_id => Some((i, t)),
                _ => None,
            });
        let Some(decl) = traits.next() else {
            anyhow::bail!("no trait named `{trait_id:?}`");
        };
        if traits.next().is_some() {
            anyhow::bail!("multiple traits named `{trait_id:?}`");
        }
        Ok(decl)
    }

    pub(crate) fn effect_alias_named(
        &self,
        id: &EffectAliasId,
        visible_crate_count: usize,
    ) -> Fallible<&EffectAlias> {
        let mut aliases = self.crates[..visible_crate_count]
            .iter()
            .flat_map(|c| &c.items)
            .filter_map(|item| match item {
                CrateItem::EffectAlias(alias) if alias.id.as_alias() == Some(id) => Some(alias),
                _ => None,
            });
        let Some(alias) = aliases.next() else {
            anyhow::bail!("unknown effect alias `{id:?}`");
        };
        if aliases.next().is_some() {
            anyhow::bail!("ambiguous effect alias `{id:?}`");
        }
        Ok(alias)
    }

    pub fn struct_named(&self, adt_id: &AdtId) -> Fallible<&Struct> {
        let mut structs: Vec<&Struct> = self
            .items_from_all_crates()
            .filter_map(|crate_item| match crate_item {
                CrateItem::AdtItem(AdtItem::Struct(s)) if s.id == *adt_id => Some(s),

                CrateItem::AdtItem(_)
                | CrateItem::FeatureGate(_)
                | CrateItem::Trait(_)
                | CrateItem::TraitImpl(_)
                | CrateItem::NegTraitImpl(_)
                | CrateItem::Fn(_)
                | CrateItem::Test(_)
                | CrateItem::EffectAlias(_) => None,
            })
            .collect();

        if structs.is_empty() {
            anyhow::bail!("no ADT named `{adt_id:?}`")
        } else if structs.len() > 1 {
            anyhow::bail!("multiple ADTs named `{adt_id:?}`")
        } else {
            Ok(structs.pop().unwrap())
        }
    }

    /// Returns the id of the crate that defines the ADT `adt_id`.
    pub fn crate_defining_adt(&self, adt_id: &AdtId) -> Fallible<&CrateId> {
        for krate in &self.crates {
            if krate
                .items
                .iter()
                .any(|item| matches!(item, CrateItem::AdtItem(a) if a.name() == adt_id))
            {
                return Ok(&krate.id);
            }
        }
        anyhow::bail!("no ADT named `{adt_id:?}`")
    }

    pub fn adt_item_named(&self, adt_id: &AdtId) -> Fallible<&AdtItem> {
        let mut adts: Vec<&AdtItem> = self
            .items_from_all_crates()
            .filter_map(|crate_item| match crate_item {
                CrateItem::AdtItem(a) if a.name() == adt_id => Some(a),

                CrateItem::AdtItem(_)
                | CrateItem::FeatureGate(_)
                | CrateItem::Trait(_)
                | CrateItem::TraitImpl(_)
                | CrateItem::NegTraitImpl(_)
                | CrateItem::Fn(_)
                | CrateItem::Test(_)
                | CrateItem::EffectAlias(_) => None,
            })
            .collect();

        if adts.is_empty() {
            anyhow::bail!("no ADT named `{adt_id:?}`")
        } else if adts.len() > 1 {
            anyhow::bail!("multiple ADTs named `{adt_id:?}`")
        } else {
            Ok(adts.pop().unwrap())
        }
    }
}
