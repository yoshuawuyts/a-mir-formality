#![allow(non_snake_case)]

use a_mir_formality::{crates, test_program_ok, FormalityTest};
use formality_core::test_util::format_error_leaves;
use formality_rust::grammar::{
    expr::{Expr, Stmt},
    Crates, Fn,
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
fn with_const_function_checks_its_body_normally() {
    let _ = test_program_ok(crates![crate Foo {
        trait Bound {}

        fn ordinary() {}

        fn annotate() with const {
            ordinary();
            with();
        }

        fn annotate_generic<T>(value: T) -> T with const where T: Bound {
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
fn with_const_function_rejects_invalid_clauses() {
    for source in [
        "fn f() with {}",
        "fn f() with Const {}",
        "fn f() with other {}",
        "fn f() with const + const {}",
    ] {
        assert!(try_term::<Fn>(source).is_err(), "accepted {source}");
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
fn with_const_block_checks_its_body_in_a_nested_scope() {
    let _ = test_program_ok(crates![crate Foo {
        fn main() -> i32 {
            let value: i32;
            with const {
                value = 1_i32;
                let value: bool = true;
            }
            return value;
        }
    }])
    .unwrap();
}

#[test]
fn with_const_block_round_trips() {
    let statement: Stmt = term("with const { prepare(); }");
    assert!(matches!(statement, Stmt::EffectBlock { .. }));
    assert_eq!(term::<Stmt>(&format!("{statement:?}")), statement);
    assert!(try_term::<Stmt>("with {}").is_err());
    assert!(try_term::<Stmt>("with Const {}").is_err());
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
fn with_const_closure_round_trips() {
    for source in [
        "|| with const {}",
        "|value: i32| with const { return value; }",
    ] {
        let expr: Expr = term(source);
        assert!(matches!(expr, Expr::EffectClosure { .. }));
        assert_eq!(term::<Expr>(&format!("{expr:?}")), expr);
    }
    assert!(try_term::<Expr>("|| with {}").is_err());
    assert!(try_term::<Expr>("|| with Const {}").is_err());
}

#[test]
fn with_const_closure_values_are_not_checked_or_executed() {
    let input = crates![crate Foo { fn main() { return || with const {}; } }];
    let error = test_program_ok(input).unwrap_err();
    assert!(format_error_leaves(&error)
        .contains("effect-annotated closure values are not supported yet"));

    let program: Crates = term(input);
    let error = formality_rust::codegen::codegen_program(&program).unwrap_err();
    assert!(error
        .to_string()
        .contains("MiniRust codegen does not support effect-annotated closures"));
}
