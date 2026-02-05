use super::assert_output;

#[test]
fn test_list_operations() {
    assert_output("list <- [1, 2, 3]\nDISPLAY(list[1])", "1");
    assert_output(
        "list <- [1, 2, 3]\nAPPEND(list, 4)\nDISPLAY(list)",
        "[1, 2, 3, 4]",
    );
    assert_output(
        "list <- [1, 2, 3]\nREMOVE(list, 2)\nDISPLAY(list)",
        "[1, 3]",
    );
    assert_output("list <- [1, 2, 3]\nDISPLAY(LENGTH(list))", "3");

    assert_output(
        r#"
            list <- [1, 2, 3]
            idx <- 1
            val <- list[idx]
            DISPLAY(val)"#,
        "1",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            item <- 4
            APPEND(list, item)
            DISPLAY(list)"#,
        "[1, 2, 3, 4]",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            idx <- 2
            REMOVE(list, idx)
            DISPLAY(list)"#,
        "[1, 3]",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            b <- REMOVE(list, 2)
            DISPLAY(b)
            "#,
        "2",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            b <- APPEND(list, 4)
            DISPLAY(b)
            "#,
        "4",
    );

    assert_output(
        r#"
            list <- [1, 3, 4]
            b <- INSERT(list, 2, 2)
            DISPLAY(b)
            "#,
        "2",
    );

    assert_output(
        r#"
            a <- [1, 2, 3]
            b <- [4, 5, 6]
            DISPLAY(a + b)
            "#,
        "[1, 2, 3, 4, 5, 6]",
    );

    assert_output(
        r#"
            empty <- []
            full <- [1, 2, 3]
            DISPLAY(empty + full)
            DISPLAY(full + empty)
            "#,
        "[1, 2, 3]\n[1, 2, 3]",
    );

    assert_output(
        r#"
            a <- [1]
            b <- [2]
            c <- [3]
            DISPLAY(a + b + c)
            "#,
        "[1, 2, 3]",
    );
}

#[test]
fn test_list_complex_operations() {
    assert_output(
        "list <- [1, 2, 3]\nlist[2] <- 5\nDISPLAY(list)",
        "[1, 5, 3]",
    );

    assert_output(
        "list <- [1, 2, 3]\nINSERT(list, 2, 5)\nDISPLAY(list)",
        "[1, 5, 2, 3]",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            INSERT(list, 2, 5)
            list[4] <- 6
            REMOVE(list, 1)
            DISPLAY(list)
            "#,
        "[5, 2, 6]",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            second <- [4, 5, 6]
            list[2] <- second[1]
            DISPLAY(list)
            "#,
        "[1, 4, 3]",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            INSERT(list, 1, 0)
            APPEND(list, 4)
            INSERT(list, 3, 2)
            DISPLAY(list)
            "#,
        "[0, 1, 2, 2, 3, 4]",
    );
}

#[test]
fn test_list_modifications() {
    assert_output(
        r#"
            nums <- [1, 2, 3, 4, 5]
            REMOVE(nums, 2)
            nums[2] <- 6
            INSERT(nums, 4, 7)
            APPEND(nums, 8)
            DISPLAY(nums)
            "#,
        "[1, 6, 4, 7, 5, 8]",
    );
}

#[test]
fn test_list_complex() {
    assert_output(
        r#"
            PROCEDURE reverseList(list) {
                result <- []
                i <- LENGTH(list)
                REPEAT LENGTH(list) TIMES {
                    APPEND(result, list[i])
                    i <- i - 1
                }
                RETURN(result)
            }
            list <- [1, 2, 3, 4]
            reversed <- reverseList(list)
            DISPLAY(reversed)
            "#,
        "[4, 3, 2, 1]",
    );

    assert_output(
        r#"
            list <- [1, 2, 3]
            APPEND(list, 4)
            INSERT(list, 2, 5)
            removed <- REMOVE(list, 3)
            DISPLAY(removed)
            DISPLAY(list)
            "#,
        "2\n[1, 5, 3, 4]",
    );
}

#[test]
fn test_list_and_string_indexing() {
    assert_output(
        r#"
            list <- [10, 20, 30]
            DISPLAY(list[1])
            DISPLAY(list[2])
            DISPLAY(list[3])
            "#,
        "10\n20\n30",
    );

    assert_output(
        r#"
            str <- "Hello"
            DISPLAY(str[1])
            DISPLAY(str[5])
            "#,
        "H\no",
    );

    assert_output(
        r#"
            list <- [1, 2, 3, 4, 5]
            idx <- 3
            DISPLAY(list[idx])
            "#,
        "3",
    );
}

#[test]
fn test_list_manipulation_with_indexes() {
    assert_output(
        r#"
            list <- [1, 2, 3]
            first <- list[1]
            last <- list[3]
            list[1] <- last
            list[3] <- first
            DISPLAY(list)
            "#,
        "[3, 2, 1]",
    );
}

#[test]
fn test_multidimensional_arrays() {
    assert_output(
        r#"
            matrix <- [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            DISPLAY(matrix[2][3])
            "#,
        "6",
    );

    assert_output(
        r#"
            mixed_matrix <- [[1, "two", TRUE], [4.5, FALSE, "six"]]
            DISPLAY(mixed_matrix[1][2])
            DISPLAY(mixed_matrix[2][1])
            "#,
        "two\n4.5",
    );

    assert_output(
        r#"
            empty_matrix <- [[""]]
            empty_matrix[1][1] <- "filled"
            DISPLAY(empty_matrix[1][1])
            "#,
        "filled",
    );
}

#[test]
fn test_three_dimensional_arrays() {
    assert_output(
        r#"
            cube <- [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
            DISPLAY(cube[1][2][1])
            DISPLAY(cube[2][1][2])
            "#,
        "3\n6",
    );
}

#[test]
fn test_matrix_operations() {
    assert_output(
        r#"
            matrix <- [[1, 2, 3], [4, 5, 6]]
            matrix[1][3] <- 10
            DISPLAY(matrix[1][3])

            matrix[2] <- [7, 8, 9]
            DISPLAY(matrix[2][2])
            "#,
        "10\n8",
    );

    assert_output(
        r#"
            matrix <- [[1, "two"], [TRUE, 4.5]]
            DISPLAY(matrix[1][2])
            DISPLAY(matrix[2][1])
            DISPLAY(matrix[2][2])
            "#,
        "two\ntrue\n4.5",
    );
}
