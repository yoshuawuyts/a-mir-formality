#![allow(non_snake_case)]

use a_mir_formality::{crates, test_program_ok, FormalityTest};
use formality_core::test_util::format_error_leaves;
use formality_rust::grammar::{
    expr::{Expr, Stmt},
    Crates, EffectList, Fn,
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
        "other",
        "unwind + other",
        "panic",
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
