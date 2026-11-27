use a_mir_formality::{crates, test_program_ok, FormalityTest};
use formality_core::test_util::format_error_leaves;

#[test]
fn i128_is_copy() {
    FormalityTest::new(crates![crate test {
        fn require_copy<T>() -> () where T: Copy { trusted }
        fn main() -> () { require_copy::<i128>(); }
    }])
    .skip_execute()
    .rustc_ok()
    .ok();
}

#[test]
fn u128_is_copy() {
    FormalityTest::new(crates![crate test {
        fn require_copy<T>() -> () where T: Copy { trusted }
        fn main() -> () { require_copy::<u128>(); }
    }])
    .skip_execute()
    .rustc_ok()
    .ok();
}

#[test]
fn trait_with_valid_fn() {
    FormalityTest::new(crates![
        crate core {
            trait A {
                fn a() -> ();
            }
        }
    ])
    .skip_execute()
    .rustc_ok()
    .ok();
}

#[test]
fn trait_impl_can_keep_or_omit_unwind_from_the_trait_signature() {
    let _ = test_program_ok(crates![
        crate Dep {
            eff reporting = async + unwind;
            trait Report {
                fn report() with reporting;
                fn quiet() with reporting;
            }
            fn may_unwind() with unwind { trusted }
        },
        crate App {
            eff reporting = async;
            eff handling = unwind;
            struct Local {}
            impl Report for Local {
                fn report() with handling { may_unwind(); }
                fn quiet() {}
            }
        }
    ])
    .unwrap();
}

#[test]
fn trait_impl_cannot_add_unwind_to_the_trait_signature() {
    let error = test_program_ok(crates![
        crate Dep {
            eff reporting = async;
            trait Report { fn report() with reporting; }
        },
        crate App {
            eff reporting = unwind;
            eff handling = unwind;
            struct Local {}
            impl Report for Local {
                fn report() with handling {}
            }
        }
    ])
    .unwrap_err();
    let diagnostic = format_error_leaves(&error);
    assert!(
        diagnostic.contains(
            "impl method `report` requires the `unwind` effect, but the trait method does not"
        ),
        "{diagnostic}"
    );
}

#[test]
fn trait_impl_body_does_not_inherit_the_trait_unwind_effect() {
    let error = test_program_ok(crates![crate Foo {
        trait Report { fn report() with unwind; }
        fn may_unwind() with unwind { trusted }
        impl Report for i32 {
            fn report() { may_unwind(); }
        }
    }])
    .unwrap_err();
    let diagnostic = format_error_leaves(&error);
    assert!(
        diagnostic.contains("call to `may_unwind` requires the `unwind` effect"),
        "{diagnostic}"
    );
}

#[test]
fn trait_with_valid_associated_type() {
    FormalityTest::new(crates![
        crate core {
            trait A {
                type Assoc : [];
            }
        }
    ])
    .skip_execute()
    .rustc_ok()
    .ok();
}

#[test]
#[ignore = "ensures bounds WF check not yet implemented, see FIXME(#228)"]
fn trait_with_ill_formed_where_clause() {
    FormalityTest::new(crates![
        crate core {
            trait A<T> where T: B {}
            trait B {}
            trait C {
                type Assoc : [ A<u32> ];
            }
        }
    ])
    .rustc_err(expect_test::expect![[r#""#]])
    .err(expect_test::expect![[r#"
            the rule "trait implied bound" at (prove_predicate.rs) failed because
              expression evaluated to an empty collection: `decls.trait_invariants()`"#]]);
}

#[test]
fn associated_type_static_outlives_bound() {
    FormalityTest::new(crates![crate Foo {
        trait A { type Item: ['static]; }
    }])
    .skip_execute()
    .rustc_ok()
    .ok();
}

#[test]
fn associated_type_outer_lifetime_outlives_bound() {
    FormalityTest::new(crates![crate Foo {
        trait A<'a> { type Item: ['a]; }
    }])
    .skip_execute()
    .rustc_ok()
    .ok();
}

#[test]
fn associated_type_trait_and_outlives_bounds() {
    FormalityTest::new(crates![crate Foo {
        trait Bar {}
        trait A<'a> { type Item: [Bar, 'a]; }
    }])
    .skip_execute()
    .rustc_ok()
    .ok();
}
