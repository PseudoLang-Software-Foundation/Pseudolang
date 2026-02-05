use super::run_test;

#[test]
fn test_error_handling() {
    assert!(run_test("DISPLAY(5 / 0)").is_err());

    assert!(run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").is_err());

    assert!(run_test("DISPLAY(undefined)").is_err());

    assert!(run_test("nonexistent(123)").is_err());
}

#[test]
fn test_try_catch() {
    use super::assert_output;

    assert_output(
        r#"
            TRY {
                DISPLAY("Before error")
                x <- 1 / 0
                DISPLAY("After error")
            } CATCH (err) {
                DISPLAY("Caught error: " + err)
            }
            "#,
        "Before error\nCaught error: Division by zero",
    );

    assert_output(
        r#"
            TRY {
                DISPLAY("No error here")
                x <- 42
            } CATCH (err) {
                DISPLAY("This won't run")
            }
            "#,
        "No error here",
    );

    assert_output(
        r#"
            TRY {
                list <- [1, 2, 3]
                DISPLAY(list[4])
            } CATCH (err) {
                DISPLAY("List error: " + err)
            }
            "#,
        "List error: List index out of bounds: 4 (size: 3)",
    );
}

#[test]
#[should_panic]
fn test_division_by_zero() {
    run_test("DISPLAY(5 / 0)").unwrap();
}

#[test]
#[should_panic(expected = "Undefined variable")]
fn test_undefined_variable() {
    run_test("DISPLAY(undefined_var)").unwrap();
}

#[test]
#[should_panic]
fn test_invalid_list_access() {
    run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").unwrap();
}

#[test]
#[should_panic(expected = "List index out of bounds")]
fn test_list_invalid_insert() {
    run_test("list <- [1, 2, 3]\nINSERT(list, 5, 4)").unwrap();
}

#[test]
#[should_panic(expected = "List index out of bounds")]
fn test_list_invalid_assignment() {
    run_test("list <- [1, 2, 3]\nlist[4] <- 5").unwrap();
}

#[test]
#[should_panic(expected = "List index out of bounds: 4 (size: 3)")]
fn test_list_index_out_of_bounds_high() {
    run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").unwrap();
}

#[test]
#[should_panic(expected = "List index out of bounds: index cannot be less than 1")]
fn test_list_index_out_of_bounds_low() {
    run_test("list <- [1, 2, 3]\nDISPLAY(list[0])").unwrap();
}

#[test]
#[should_panic(expected = "String index out of bounds: 3 (size: 2)")]
fn test_string_index_out_of_bounds_high() {
    run_test(
        r#"str <- "hi"
DISPLAY(str[3])"#,
    )
    .unwrap();
}

#[test]
#[should_panic(expected = "String index out of bounds: index cannot be less than 1")]
fn test_string_index_out_of_bounds_low() {
    run_test(
        r#"str <- "hi"
DISPLAY(str[0])"#,
    )
    .unwrap();
}

#[test]
fn test_string_indexing_edge_cases() {
    use super::assert_output;

    assert_output(
        r#"
            str <- "A"
            DISPLAY(str[1])
            "#,
        "A",
    );

    assert!(run_test(r#"str <- ""\nDISPLAY(str[1])"#).is_err());
}
