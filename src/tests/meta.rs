//! Meta programming: inspecting types, running generated source, reaching
//! variables and procedures by name.

use super::{assert_output, get_error, run_test};

// ---------------------------------------------------------------------------
// TYPEOF
// ---------------------------------------------------------------------------

#[test]
fn test_typeof_every_value_kind() {
    assert_output(
        r#"
        DISPLAY(TYPEOF(1))
        DISPLAY(TYPEOF(-1))
        DISPLAY(TYPEOF(1.5))
        DISPLAY(TYPEOF("s"))
        DISPLAY(TYPEOF(TRUE))
        DISPLAY(TYPEOF(FALSE))
        DISPLAY(TYPEOF([1, 2]))
        DISPLAY(TYPEOF([]))
        DISPLAY(TYPEOF({"a": 1}))
        DISPLAY(TYPEOF({}))
        DISPLAY(TYPEOF(NULL))
        DISPLAY(TYPEOF(NAN))
        "#,
        "integer\ninteger\nfloat\nstring\nboolean\nboolean\nlist\nlist\ndictionary\ndictionary\nnull\nnan",
    );
}

#[test]
fn test_typeof_an_expression_not_just_a_literal() {
    assert_output(
        r#"
        DISPLAY(TYPEOF(1 + 1))
        DISPLAY(TYPEOF(1 / 2))
        DISPLAY(TYPEOF(1.0 / 2))
        DISPLAY(TYPEOF(1 = 1))
        DISPLAY(TYPEOF(SPLIT("a b", " ")))
        DISPLAY(TYPEOF(TOSTRING(1)))
        DISPLAY(TYPEOF(TONUM("1")))
        "#,
        // Integer division stays integer here; a float operand is what makes the
        // result one.
        "integer\ninteger\nfloat\nboolean\nlist\nstring\ninteger",
    );
}

#[test]
fn test_typeof_arity() {
    let err = get_error("DISPLAY(TYPEOF())");
    assert!(err.contains("TYPEOF requires 1 argument"), "{}", err);

    let err = get_error("DISPLAY(TYPEOF(1, 2))");
    assert!(err.contains("TYPEOF requires 1 argument"), "{}", err);
}

// ---------------------------------------------------------------------------
// EXECUTE
// ---------------------------------------------------------------------------

#[test]
fn test_execute_runs_statements_in_the_calling_scope() {
    assert_output(
        r#"
        EXECUTE("made <- 41")
        DISPLAY(made)
        EXECUTE("made <- made + 1")
        DISPLAY(made)
        "#,
        "41\n42",
    );
}

#[test]
fn test_execute_can_declare_a_procedure() {
    assert_output(
        r#"
        source <- "PROCEDURE twice(n)" + "\n" + "{" + "\n" + "RETURN n * 2" + "\n" + "}"
        EXECUTE(source)
        DISPLAY(twice(21))
        DISPLAY(CONTAINS(PROCEDURES(), "twice"))
        "#,
        "42\ntrue",
    );
}

#[test]
fn test_execute_runs_multiple_statements_and_can_display() {
    assert_output(
        r#"
        EXECUTE("DISPLAY(\"from inside\")" + "\n" + "DISPLAY(2 + 2)")
        DISPLAY("after")
        "#,
        "from inside\n4\nafter",
    );
}

#[test]
fn test_execute_sees_variables_already_defined() {
    assert_output(
        r#"
        outer <- 10
        EXECUTE("DISPLAY(outer * 2)")
        "#,
        "20",
    );
}

#[test]
fn test_execute_returns_nothing_itself() {
    assert_output("DISPLAY(TYPEOF(EXECUTE(\"x <- 1\")))", "unit");
}

#[test]
fn test_execute_reports_a_syntax_error_in_its_source() {
    let err = get_error("EXECUTE(\"IF\")");
    assert!(err.contains("EXECUTE could not parse"), "{}", err);
}

#[test]
fn test_execute_propagates_a_runtime_error_and_it_is_catchable() {
    let output = run_test(
        r#"
        TRY
        {
            EXECUTE("y <- 1 / 0")
            DISPLAY("not reached")
        } CATCH (err)
        {
            DISPLAY("caught")
        }
        "#,
    )
    .expect("TRY/CATCH around EXECUTE");
    assert_eq!(output, "caught");
}

#[test]
fn test_execute_requires_a_string() {
    let err = get_error("EXECUTE(1)");
    assert!(err.contains("EXECUTE requires a string"), "{}", err);
}

#[test]
fn test_self_referential_execute_hits_the_recursion_guard() {
    // Without a stack frame per nested EXECUTE this recursion would bypass
    // MAX_STACK_DEPTH entirely and abort the process on a real stack overflow.
    let err = get_error(
        r#"
        code <- "EXECUTE(code)"
        EXECUTE(code)
        "#,
    );
    assert!(
        err.contains("Maximum EXECUTE nesting depth exceeded"),
        "{}",
        err
    );
}

#[test]
fn test_self_referential_eval_hits_the_recursion_guard() {
    let err = get_error(
        r#"
        code <- "EVAL(code)"
        DISPLAY(EVAL(code))
        "#,
    );
    assert!(
        err.contains("Maximum EVAL nesting depth exceeded"),
        "{}",
        err
    );
}

#[test]
fn test_mutually_recursive_eval_and_execute_are_also_bounded() {
    // The two share one counter, so bouncing between them cannot get past the
    // limit either.
    let err = get_error(
        r#"
        a <- "EXECUTE(b)"
        b <- "DISPLAY(EVAL(\"1\")) " + "\n" + "EXECUTE(a)"
        EXECUTE(a)
        "#,
    );
    assert!(err.contains("nesting depth exceeded"), "{}", err);
}

#[test]
fn test_nesting_below_the_limit_still_works() {
    // The guard must not get in the way of legitimate nesting.
    assert_output(
        r#"
        DISPLAY(EVAL("EVAL(\"EVAL(\\\"1 + 1\\\")\")"))
        "#,
        "2",
    );
}

#[test]
fn test_eval_still_evaluates_expressions() {
    assert_output(
        r#"
        x <- 3
        DISPLAY(EVAL("x * (x + 1)"))
        DISPLAY(EVAL("2 + 2"))
        "#,
        "12\n4",
    );
}

// ---------------------------------------------------------------------------
// Reaching variables by name
// ---------------------------------------------------------------------------

#[test]
fn test_isdefined() {
    assert_output(
        r#"
        DISPLAY(ISDEFINED("nothing"))
        thing <- 1
        DISPLAY(ISDEFINED("thing"))
        "#,
        "false\ntrue",
    );
}

#[test]
fn test_isdefined_sees_the_builtin_argument_variables() {
    assert_output(
        r#"
        DISPLAY(ISDEFINED("ARGS"))
        DISPLAY(ISDEFINED("ARGCOUNT"))
        "#,
        "true\ntrue",
    );
}

#[test]
fn test_getvar_and_setvar_roundtrip() {
    assert_output(
        r#"
        SETVAR("made", 7)
        DISPLAY(made)
        DISPLAY(GETVAR("made"))
        SETVAR("made", "now a string")
        DISPLAY(GETVAR("made"))
        "#,
        "7\n7\nnow a string",
    );
}

#[test]
fn test_setvar_returns_the_value_assigned() {
    assert_output("DISPLAY(SETVAR(\"v\", 5))", "5");
}

#[test]
fn test_setvar_accepts_any_value_kind() {
    assert_output(
        r#"
        SETVAR("l", [1, 2])
        SETVAR("d", {"k": 1})
        DISPLAY(l[2])
        DISPLAY(d["k"])
        "#,
        "2\n1",
    );
}

#[test]
fn test_getvar_default_and_error() {
    assert_output("DISPLAY(GETVAR(\"missing\", \"dflt\"))", "dflt");

    let err = get_error("DISPLAY(GETVAR(\"missing\"))");
    assert!(err.contains("Variable 'missing' is not defined"), "{}", err);
}

#[test]
fn test_a_default_expression_may_have_a_side_effect() {
    // The lookup must not still hold a shared borrow of the environment while the
    // default is evaluated: a default that assigns needs a mutable one, and
    // holding both used to abort the interpreter with "RefCell already borrowed".
    assert_output(
        r#"
        DISPLAY(GETVAR("missing", SETVAR("madebydefault", 1)))
        DISPLAY(madebydefault)
        DISPLAY(GETENV("PSL_TEST_NOT_SET_XYZ", SETVAR("alsomade", 2)))
        DISPLAY(alsomade)
        "#,
        "1\n1\n2\n2",
    );
}

#[test]
fn test_nested_setvar_and_dynamic_call_arguments_are_safe() {
    assert_output(
        r#"
        PROCEDURE identity(v)
        {
            RETURN v
        }
        DISPLAY(SETVAR("outer", SETVAR("inner", 3)))
        DISPLAY(inner)
        DISPLAY(CALL("identity", [SETVAR("viacall", 4)]))
        DISPLAY(viacall)
        "#,
        "3\n3\n4\n4",
    );
}

#[test]
fn test_setvar_rejects_a_name_no_source_could_use() {
    // Keywords included: `SETVAR("IF", 1)` used to succeed and leave a binding that
    // `DISPLAY(IF)` could never read.
    for bad in [
        "\"has space\"",
        "\"1leading\"",
        "\"has-dash\"",
        "\"\"",
        "\"IF\"",
        "\"RETURN\"",
        "\"TRUE\"",
        "\"NULL\"",
        "\"MOD\"",
        "\"PROCEDURE\"",
    ] {
        let err = get_error(&format!("SETVAR({}, 1)", bad));
        assert!(
            err.contains("not a usable variable name"),
            "{}: {}",
            bad,
            err
        );
    }
}

#[test]
fn test_setvar_accepts_underscores_and_digits_after_the_first_letter() {
    assert_output(
        r#"
        SETVAR("a_1", "ok")
        DISPLAY(a_1)
        "#,
        "ok",
    );
}

#[test]
fn test_unsetvar_removes_a_binding() {
    assert_output(
        r#"
        gone <- 1
        DISPLAY(UNSETVAR("gone"))
        DISPLAY(ISDEFINED("gone"))
        DISPLAY(UNSETVAR("never-existed"))
        "#,
        "true\nfalse\nfalse",
    );
}

#[test]
fn test_unsetvar_only_removes_from_the_current_scope() {
    // `unset` used to walk the parent chain, so a procedure could delete a caller's
    // variable -- something no assignment can do, since SETVAR writes the current
    // scope.
    assert_output(
        r#"
        outer <- "kept"
        PROCEDURE tryremove()
        {
            RETURN UNSETVAR("outer")
        }
        DISPLAY(tryremove())
        DISPLAY(outer)
        DISPLAY(ISDEFINED("outer"))
        "#,
        "false\nkept\ntrue",
    );
}

#[test]
fn test_unsetvar_removes_a_binding_made_in_the_same_scope() {
    assert_output(
        r#"
        PROCEDURE roundtrip()
        {
            local <- 1
            DISPLAY(UNSETVAR("local"))
            RETURN ISDEFINED("local")
        }
        DISPLAY(roundtrip())
        "#,
        "true\nfalse",
    );
}

#[test]
fn test_variables_lists_what_is_defined_and_is_sorted() {
    assert_output(
        r#"
        zebra <- 1
        alpha <- 2
        names <- VARIABLES()
        DISPLAY(TYPEOF(names))
        DISPLAY(CONTAINS(names, "alpha"))
        DISPLAY(CONTAINS(names, "zebra"))
        DISPLAY(CONTAINS(names, "nothing"))
        DISPLAY(names[1] < names[LENGTH(names)])
        "#,
        "list\ntrue\ntrue\nfalse\ntrue",
    );
}

#[test]
fn test_variables_inside_a_procedure_sees_both_scopes() {
    assert_output(
        r#"
        outer <- 1
        PROCEDURE look(param)
        {
            local <- 2
            DISPLAY(CONTAINS(VARIABLES(), "local"))
            DISPLAY(CONTAINS(VARIABLES(), "param"))
            DISPLAY(CONTAINS(VARIABLES(), "outer"))
        }
        look(9)
        "#,
        "true\ntrue\ntrue",
    );
}

#[test]
fn test_procedures_lists_declared_procedures_sorted() {
    assert_output(
        r#"
        PROCEDURE zeta()
        {
            RETURN 1
        }
        PROCEDURE alpha()
        {
            RETURN 2
        }
        names <- PROCEDURES()
        DISPLAY(names)
        "#,
        "[alpha, zeta]",
    );
}

#[test]
fn test_procedures_is_empty_when_none_are_declared() {
    assert_output("DISPLAY(PROCEDURES())", "[]");
}

// ---------------------------------------------------------------------------
// CALL
// ---------------------------------------------------------------------------

#[test]
fn test_call_dispatches_by_name() {
    assert_output(
        r#"
        PROCEDURE double(n)
        {
            RETURN n * 2
        }
        DISPLAY(CALL("double", [21]))
        "#,
        "42",
    );
}

#[test]
fn test_call_with_several_arguments_and_with_none() {
    assert_output(
        r#"
        PROCEDURE add3(a, b, c)
        {
            RETURN a + b + c
        }
        PROCEDURE greet()
        {
            RETURN "hi"
        }
        DISPLAY(CALL("add3", [1, 2, 3]))
        DISPLAY(CALL("greet"))
        DISPLAY(CALL("greet", []))
        "#,
        "6\nhi\nhi",
    );
}

#[test]
fn test_call_with_a_name_chosen_at_runtime() {
    assert_output(
        r#"
        PROCEDURE up(s)
        {
            RETURN UPPERCASE(s)
        }
        PROCEDURE down(s)
        {
            RETURN LOWERCASE(s)
        }
        FOR EACH which IN ["up", "down"]
        {
            DISPLAY(CALL(which, ["MiXeD"]))
        }
        "#,
        "MIXED\nmixed",
    );
}

#[test]
fn test_call_passes_values_not_expressions() {
    assert_output(
        r#"
        PROCEDURE describe(v)
        {
            RETURN TYPEOF(v)
        }
        DISPLAY(CALL("describe", [[1, 2]]))
        DISPLAY(CALL("describe", [{"a": 1}]))
        "#,
        "list\ndictionary",
    );
}

#[test]
fn test_call_on_a_missing_procedure_errors_helpfully() {
    let err = get_error("DISPLAY(CALL(\"nope\", []))");
    assert!(
        err.contains("could not find a procedure named 'nope'"),
        "{}",
        err
    );
    assert!(err.contains("built-in"), "{}", err);
}

#[test]
fn test_call_rejects_a_non_list_argument_bundle() {
    let err = get_error(
        r#"
        PROCEDURE p(a)
        {
            RETURN a
        }
        DISPLAY(CALL("p", "notalist"))
        "#,
    );
    assert!(err.contains("list of arguments"), "{}", err);
}

#[test]
fn test_call_recursion_is_still_bounded() {
    let err = get_error(
        r#"
        PROCEDURE forever(n)
        {
            RETURN CALL("forever", [n + 1])
        }
        DISPLAY(forever(0))
        "#,
    );
    assert!(err.contains("maximum recursion depth exceeded"), "{}", err);
}

#[test]
fn test_call_errors_inside_the_callee_are_catchable() {
    let output = run_test(
        r#"
        PROCEDURE boom()
        {
            RETURN 1 / 0
        }
        TRY
        {
            DISPLAY(CALL("boom", []))
        } CATCH (err)
        {
            DISPLAY("caught")
        }
        "#,
    )
    .expect("TRY/CATCH around CALL");
    assert_eq!(output, "caught");
}

// ---------------------------------------------------------------------------
// Built-ins shadow procedures, and the new names are no exception
// ---------------------------------------------------------------------------

#[test]
fn test_new_builtins_shadow_a_user_procedure_of_the_same_name() {
    assert_output(
        r#"
        PROCEDURE TYPEOF(v)
        {
            RETURN "from the procedure"
        }
        DISPLAY(TYPEOF(1))
        "#,
        "integer",
    );
}

#[test]
fn test_a_variable_may_still_be_named_like_a_builtin() {
    // Only *calls* resolve to built-ins, so a plain variable read is unaffected.
    assert_output(
        r#"
        VERSION <- "mine"
        DISPLAY(VERSION)
        DISPLAY(VERSION())
        "#,
        &format!("mine\n{}", env!("CARGO_PKG_VERSION")),
    );
}

// ---------------------------------------------------------------------------
// The built-in name list behind the undefined-variable hint
// ---------------------------------------------------------------------------

#[test]
fn test_every_builtin_is_listed_for_the_undefined_variable_hint() {
    // `BUILTIN_NAMES` is a second copy of the dispatcher's arms, so this reads the
    // arms out of the source and fails when the list falls behind. It also checks
    // the sort order, which `binary_search` depends on.
    let source = include_str!("../interpreter.rs");
    let dispatcher = source
        .split_once("fn eval_builtin(")
        .expect("eval_builtin exists")
        .1;
    let dispatcher = dispatcher
        .split_once("\n        _ => None,")
        .expect("the dispatcher ends with a catch-all")
        .0;

    let mut dispatched: Vec<&str> = dispatcher
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix('"')?;
            let (name, tail) = rest.split_once('"')?;
            tail.trim_start().starts_with("=>").then_some(name)
        })
        .filter(|name| name.chars().all(|c| c.is_ascii_uppercase() || c == '_'))
        .collect();
    dispatched.sort_unstable();
    dispatched.dedup();
    assert!(
        dispatched.len() > 100,
        "only found {} dispatched names, the parse above must have broken",
        dispatched.len()
    );

    // Read the list out of the source too, so nothing is exposed just for this test.
    let listing = source
        .split_once("const BUILTIN_NAMES: &[&str] = &[")
        .expect("BUILTIN_NAMES exists")
        .1
        .split_once("];")
        .expect("the list is closed")
        .0;
    let listed: Vec<&str> = listing
        .split(',')
        .filter_map(|entry| entry.trim().strip_prefix('"')?.strip_suffix('"'))
        .collect();
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    assert_eq!(listed, sorted, "BUILTIN_NAMES must be sorted");

    let missing: Vec<&&str> = dispatched.iter().filter(|n| !listed.contains(n)).collect();
    assert!(missing.is_empty(), "not in BUILTIN_NAMES: {:?}", missing);

    let extra: Vec<&&str> = listed.iter().filter(|n| !dispatched.contains(n)).collect();
    assert!(
        extra.is_empty(),
        "in BUILTIN_NAMES but not dispatched: {:?}",
        extra
    );
}

#[test]
fn test_a_zero_argument_builtin_used_without_parentheses_says_so() {
    for name in ["CWD", "SYSINFO", "PID", "VERSION", "MODULES"] {
        let err = get_error(&format!("DISPLAY({})", name));
        assert!(
            err.contains("is a built-in function") && err.contains(&format!("{}()", name)),
            "{}: {}",
            name,
            err
        );
    }
}

#[test]
fn test_an_ordinary_undefined_variable_gets_no_hint() {
    let err = get_error("DISPLAY(notabuiltin)");
    assert!(err.contains("Undefined variable: notabuiltin"), "{}", err);
    assert!(!err.contains("built-in function"), "{}", err);
}
