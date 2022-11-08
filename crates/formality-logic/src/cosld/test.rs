#![cfg(test)]

use formality_infer::Env;
use formality_macros::test;
use formality_types::{
    collections::Set,
    db::mock::MockDatabase,
    grammar::{AtomicRelation, Binder, Goal},
    parse::term,
};

use super::CosldResult;

#[test]
fn simple_test() {
    let db = MockDatabase::new()
        .with_program_clause(
            "for_all(<ty T> implies([is_implemented(Debug(T))], is_implemented(Debug(Vec<T>))))",
        )
        .with_program_clause("is_implemented(Debug(u32))")
        .into_db();
    let env = Env::default();

    let results = super::prove(&db, &env, &[], &term("is_implemented(Debug(Vec<u32>))"));

    expect_test::expect![[r#"
            {
                yes(
                    env(
                        U(0),
                        [
                            inference_var_data(
                                ty,
                                U(0),
                                Some(
                                    (rigid (scalar u32)),
                                ),
                                [],
                                [],
                                [],
                                [],
                            ),
                        ],
                        no,
                    ),
                ),
            }
        "#]]
    .assert_debug_eq(&results);
}

fn extract_relations(s: &Set<CosldResult>) -> Vec<Option<Vec<AtomicRelation>>> {
    s.iter()
        .map(|r| match r {
            CosldResult::Maybe => None,
            CosldResult::Yes(env) => Some(env.inference_var_relations()),
        })
        .collect()
}

#[test]
fn outlives_refs() {
    let db = MockDatabase::new().into_db();
    let mut env = Env::default();

    let b_goal: Binder<Goal> = term("<lt a, lt b> sub(&a u32, &b u32)");
    let goal = env.instantiate_existentially(&b_goal);
    let results = super::prove(&db, &env, &[], &goal);

    expect_test::expect![[r#"
        [
            Some(
                [
                    outlives(
                        ?0,
                        ?1,
                    ),
                ],
            ),
        ]
    "#]]
    .assert_debug_eq(&extract_relations(&results));
}

#[test]
fn outlives_assoc_type() {
    let db = MockDatabase::new().into_db();
    let mut env = Env::default();

    let b_goal: Binder<Goal> = term("<lt a, lt b> outlives(<u32 as Foo<a>>::Item, b)");
    let goal = env.instantiate_existentially(&b_goal);
    let results = super::prove(&db, &env, &[], &goal);

    expect_test::expect![[r#"
        [
            Some(
                [
                    outlives(
                        ?0,
                        ?1,
                    ),
                ],
            ),
        ]
    "#]]
    .assert_debug_eq(&extract_relations(&results));
}

#[test]
fn outlives_assoc_type_normalizes() {
    let db = MockDatabase::new()
        .with_program_clause("for_all(<ty T, lt a> normalizes_to((alias (Foo::Item) T a), T))")
        .into_db();
    let mut env = Env::default();

    let b_goal: Binder<Goal> = term("<lt a, lt b> outlives(<u32 as Foo<a>>::Item, b)");
    eprintln!("{b_goal:?}");
    let goal = env.instantiate_existentially(&b_goal);
    let results = super::prove(&db, &env, &[], &goal);

    // The first result is when we successfully normalize.
    // Note that there are no outlives obligations.
    // The second result is when we do NOT normalize.
    // We do produce outlives obligations.
    expect_test::expect![[r#"
        [
            Some(
                [
                    equals(
                        ?2,
                        (rigid (scalar u32)),
                    ),
                    equals(
                        ?3,
                        (rigid (scalar u32)),
                    ),
                    equals(
                        ?4,
                        ?0,
                    ),
                ],
            ),
            Some(
                [
                    outlives(
                        ?0,
                        ?1,
                    ),
                ],
            ),
        ]
    "#]]
    .assert_debug_eq(&extract_relations(&results));
}
