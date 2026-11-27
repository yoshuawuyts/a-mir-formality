#![allow(non_snake_case)]

use a_mir_formality::{crates, test_program_ok, FormalityTest};
use formality_core::test_util::format_error_leaves;
use formality_rust::grammar::{
    expr::{Expr, Stmt},
    CrateItem, Crates, EffectList, Fn,
};
use formality_rust::rust::{term, try_term};

#[test]
fn ok() {
    FormalityTest::new(crates![crate Foo {
        // fn simple_fn() {}
        fn simple_fn() -> () { trusted }

        // fn one_arg<T>(_: T) {}
        fn one_arg<T>(v0: T) -> () { trusted }

        // fn one_ret<T>(_: T) {}
        fn one_ret<T>() -> T { trusted }

        // fn arg_ret<T, U>(_: T) -> U {}
        fn arg_ret<T, U>(v0: T) -> U { trusted }

        // fn multi_arg_ret<T, Y, U, I>(_: T, _: Y) -> (U, I) {}
        fn multi_arg_ret<T, Y, U, I>(v0: T, v1: Y) -> (U, I) { trusted }
    }])
    .skip_execute()
    .rustc_ok()
    .ok()
}

#[test]
fn lifetime() {
    FormalityTest::new(crates![crate Foo {
        // fn one_lt_arg<'a, T>(_: &'a T) -> () {}
        fn one_lt_arg<'a, T>(v0: &'a T) -> ()
        where
            T: 'a, // FIXME(#202): Implied bounds should not have to be explicit
        { trusted }
    }])
    .skip_execute()
    .rustc_ok()
    .ok()
}

#[test]
fn omitted_return_type() {
    FormalityTest::new(crates![
        crate Foo {
            fn simple_fn() { trusted }
            fn one_arg<T>(v0: T) { trusted }
            fn with_where<'a, T>(v0: &'a T)
            where
                T: 'a,
            { trusted }
        }
    ])
    .skip_execute()
    .ok()
}

#[test]
fn omitted_return_type_executes() {
    FormalityTest::new(crates![crate test {
        fn helper() {
            println!(42_i32);
        }

        fn main() {
            helper();
        }
    }])
    .expect_output("42\n")
    .ok()
}

#[test]
fn with_effects_function_checks_its_body_normally() {
    let _ = test_program_ok(crates![crate Foo {
        trait Bound {}

        fn ordinary() {}
        fn async() {}
        fn unwind() {}

        fn annotate() with const {
            ordinary();
            with();
            async();
            unwind();
        }

        fn annotate_generic<T>(value: T) -> T where T: Bound with unwind + const + async {
            return value;
        }

        fn with() {}
    }])
    .unwrap();
}

#[test]
fn with_const_function_round_trips() {
    let function: Fn = term("fn answer() -> i32 with const { trusted }");
    assert_eq!(function.binder.peek().effects, term("const"));
    assert_eq!(term::<Fn>(&format!("{function:?}")), function);

    let plain: Fn = term("fn answer() -> i32 { trusted }");
    assert!(plain.binder.peek().effects.is_empty());
    assert_eq!(term::<Fn>(&format!("{plain:?}")), plain);
}

#[test]
fn function_with_clause_follows_where_clause() {
    let function: Fn = term("fn f() -> i32 where i32: Bound with const { trusted }");
    let signature = function.binder.peek();
    assert_eq!(signature.where_clauses.len(), 1);
    assert_eq!(signature.effects, term("const"));

    let printed = format!("{function:?}");
    let (before_with, _) = printed.split_once(" with const").unwrap();
    assert!(before_with.contains(" where "), "{printed}");
    assert_eq!(term::<Fn>(&printed), function);

    assert!(try_term::<Fn>("fn f() -> i32 with const where i32: Bound { trusted }").is_err());
}

#[test]
fn with_effects_function_round_trips_in_source_order() {
    let function: Fn = term("fn answer() -> i32 with unwind+const + async { trusted }");
    assert_eq!(function.binder.peek().effects, term("unwind + const+async"));
    let printed = format!("{function:?}");
    assert!(printed.contains("with unwind + const + async"), "{printed}");
    assert_eq!(term::<Fn>(&printed), function);
}

#[test]
fn with_effects_rejects_invalid_clauses() {
    for list in [
        "",
        "Const",
        "const + Async",
        "other + const",
        "other + other",
        "unwind + other",
        "const + panic",
        "+ const",
        "const +",
        "const ++ async",
        "const async",
        "const, async",
        "(const + async)",
    ] {
        let block = format!("with {list} {{}}");
        let function = format!("fn f() {block}");
        let closure = format!("|| {block}");
        assert!(try_term::<Fn>(&function).is_err(), "accepted {function}");
        assert!(try_term::<Stmt>(&block).is_err(), "accepted {block}");
        assert!(try_term::<Expr>(&closure).is_err(), "accepted {closure}");
    }
}

#[test]
fn with_effects_reports_duplicate_names() {
    for (list, duplicate) in [
        ("const + const", "const"),
        ("const + unwind + const", "const"),
        ("const + unwind + unwind", "unwind"),
    ] {
        let block = format!("with {list} {{}}");
        let function = format!("fn f() {block}");
        let closure = format!("|| {block}");
        let expected = format!("duplicate built-in effect `{duplicate}`");

        for (source, error) in [
            (&function, try_term::<Fn>(&function).err()),
            (&block, try_term::<Stmt>(&block).err()),
            (&closure, try_term::<Expr>(&closure).err()),
        ] {
            let error = error.unwrap_or_else(|| panic!("accepted {source}"));
            assert!(error.to_string().contains(&expected), "{source}: {error}");
        }
    }
}

#[test]
fn with_const_function_is_not_executable() {
    let program: Crates = term(crates![crate Foo { fn main() with const {} }]);
    let minirust_error = formality_rust::codegen::codegen_program(&program).unwrap_err();
    assert!(minirust_error
        .to_string()
        .contains("MiniRust codegen does not support `with const` on functions"));
}

#[test]
fn with_effects_block_checks_its_body_in_a_nested_scope() {
    let _ = test_program_ok(crates![crate Foo {
        fn main() -> i32 {
            let value: i32;
            with async + unwind {
                value = 1_i32;
                let value: bool = true;
            }
            return value;
        }
    }])
    .unwrap();
}

#[test]
fn blocks_round_trip_with_or_without_effects() {
    for (source, expected) in [
        ("{ prepare(); }", EffectList::default()),
        ("with const { prepare(); }", term("const")),
        ("with async {}", term("async")),
        ("with async + unwind { prepare(); }", term("async + unwind")),
    ] {
        let statement: Stmt = term(source);
        let Stmt::Block { effects, .. } = &statement else {
            panic!("expected a block: {statement:?}");
        };
        assert_eq!(effects, &expected);
        assert_eq!(term::<Stmt>(&format!("{statement:?}")), statement);
    }
}

#[test]
fn with_const_block_is_not_executable() {
    let program: Crates = term(crates![crate Foo { fn main() { with const {} } }]);
    let error = formality_rust::codegen::codegen_program(&program).unwrap_err();
    assert!(error
        .to_string()
        .contains("MiniRust codegen does not support `with const` on blocks"));
}

#[test]
fn closures_round_trip_with_or_without_effects() {
    for (source, expected) in [
        ("|| {}", EffectList::default()),
        ("|| with const {}", term("const")),
        ("|| with unwind {}", term("unwind")),
        ("|value: i32| { return value; }", EffectList::default()),
        ("|value: i32| with const { return value; }", term("const")),
        (
            "|value: i32| with const + async { return value; }",
            term("const + async"),
        ),
    ] {
        let expr: Expr = term(source);
        let Expr::Closure { effects, .. } = &expr else {
            panic!("expected a closure: {expr:?}");
        };
        assert_eq!(effects, &expected);
        assert_eq!(term::<Expr>(&format!("{expr:?}")), expr);
    }
}

#[test]
fn closure_values_are_not_checked_or_executed() {
    for input in [
        crates![crate Foo { fn main() { return || {}; } }],
        crates![crate Foo { fn main() { return || with const {}; } }],
        crates![crate Foo { fn main() { return || with unwind + const {}; } }],
    ] {
        let error = test_program_ok(input).unwrap_err();
        assert!(format_error_leaves(&error).contains("closure values are not supported yet"));

        let program: Crates = term(input);
        let error = formality_rust::codegen::codegen_program(&program).unwrap_err();
        assert!(error
            .to_string()
            .contains("MiniRust codegen does not support closures"));
    }
}

#[test]
fn with_effect_aliases_round_trip() {
    let program: Crates = term(crates![crate Foo {
        eff reporting = async + unwind;

        fn report() with reporting {
            with reporting {}
            return |value: i32| with reporting { return value; };
        }
    }]);
    let printed = format!("{program:?}");
    assert!(
        printed.contains("eff reporting = async + unwind"),
        "{printed}"
    );
    assert_eq!(printed.matches("with reporting").count(), 3, "{printed}");
    assert_eq!(term::<Crates>(&printed), program);
}

#[test]
fn with_effect_aliases_resolve_locally_and_in_dependencies() {
    let _ = test_program_ok(crates![
        crate Dep {
            eff reporting = async + unwind;
        },
        crate App {
            trait Report {
                fn report() with reporting;
            }

            impl Report for i32 {
                fn report() with eff {
                    with reporting {
                        with();
                        eff();
                    }
                }
            }

            fn with() {}
            fn eff() {}
            fn async() with async {}
            fn unwind() with unwind {}

            fn main() -> i32 with async_report {
                async();
                unwind();
                let value: i32;
                with eff {
                    value = 1_i32;
                    let value: bool = true;
                }
                return value;
            }

            eff eff = const;
            eff async_report = async + unwind;
        }
    ])
    .unwrap();
}

#[test]
fn with_effect_aliases_reject_unknown_names() {
    let check = |input: &str, name: &str| {
        let error = test_program_ok(input).unwrap_err();
        let diagnostic = format_error_leaves(&error);
        assert!(
            diagnostic.contains(&format!("unknown effect alias `{name}`")),
            "{input}\n{diagnostic}"
        );
    };

    for input in [
        crates![crate Foo { fn missing() {} fn main() with missing {} }],
        crates![crate Foo { trait Report { fn report() with missing; } }],
        crates![crate Foo {
            trait Report { fn report(); }
            impl Report for i32 { fn report() with missing {} }
        }],
        crates![crate Foo { fn main() { with missing {} } }],
        crates![crate Foo { fn main() { return || with missing {}; } }],
        crates![
            crate Early { fn main() with missing {} },
            crate Later { eff missing = const; }
        ],
    ] {
        check(input, "missing");
    }
    check(crates![crate Foo { fn main() with panic {} }], "panic");
}

#[test]
fn with_effect_aliases_require_a_unique_visible_definition() {
    let _ = test_program_ok(crates![
        crate Early { eff reporting = async; fn early() with reporting {} },
        crate Later { eff reporting = unwind; }
    ])
    .unwrap();

    for input in [
        crates![
            crate Early { eff reporting = async; },
            crate Later { eff reporting = unwind; fn later() with reporting {} }
        ],
        crates![
            crate Early { eff reporting = async; },
            crate Later { eff reporting = const; fn later() { return || with reporting {}; } }
        ],
    ] {
        let error = test_program_ok(input).unwrap_err();
        let diagnostic = format_error_leaves(&error);
        assert!(
            diagnostic.contains("ambiguous effect alias `reporting`"),
            "{input}\n{diagnostic}"
        );
        assert!(
            !diagnostic.contains("closure values are not supported yet"),
            "{input}\n{diagnostic}"
        );
    }
}

#[test]
fn with_effect_aliases_reject_duplicate_or_reserved_declarations() {
    let error =
        test_program_ok(crates![crate Foo { eff reporting = const; eff reporting = async; }])
            .unwrap_err();
    let diagnostic = format_error_leaves(&error);
    assert!(
        diagnostic.contains("the effect alias name `reporting` is defined multiple times"),
        "{diagnostic}"
    );

    for builtin in ["const", "async", "unwind"] {
        let error =
            test_program_ok(&format!("[crate Foo {{ eff {builtin} = const; }}]")).unwrap_err();
        let diagnostic = format_error_leaves(&error);
        assert!(
            diagnostic.contains(&format!(
                "effect alias `{builtin}` conflicts with a built-in effect"
            )),
            "{diagnostic}"
        );
    }
}

#[test]
fn with_effect_aliases_reject_invalid_declarations() {
    for input in [
        "eff reporting = ;",
        "eff reporting = logging;",
        "eff reporting = panic;",
        "eff reporting = async + logging;",
        "eff reporting = const",
        "eff Reporting = const;",
        "eff reportIng = const;",
    ] {
        assert!(try_term::<CrateItem>(input).is_err(), "accepted {input}");
    }

    assert!(try_term::<Fn>("fn f() { eff reporting = async; }").is_err());
    let error = try_term::<CrateItem>("eff reporting = const + const;").unwrap_err();
    assert!(error
        .to_string()
        .contains("duplicate built-in effect `const`"));
}

#[test]
fn with_effect_aliases_do_not_enable_closure_values() {
    let error = test_program_ok(crates![
        crate Dep { eff reporting = unwind; },
        crate App { fn main() { return || with reporting {}; } }
    ])
    .unwrap_err();
    let diagnostic = format_error_leaves(&error);
    assert!(
        diagnostic.contains("closure values are not supported yet"),
        "{diagnostic}"
    );
}
