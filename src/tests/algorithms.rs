use super::assert_output;

#[test]
fn test_sort() {
    assert_output(
        "list <- [3, 1, 4, 1, 5]\nDISPLAY(SORT(list))",
        "[1, 1, 3, 4, 5]",
    );

    assert_output(
        r#"
            list <- [3, 1, 4, 1, 5]
            sorted <- SORT(list)
            DISPLAY(sorted)"#,
        "[1, 1, 3, 4, 5]",
    );
}

#[test]
fn test_bubble_sort() {
    assert_output(
        r#"
                    PROCEDURE bubbleSort(aList)
                    {
                        n <- LENGTH(aList)
                        REPEAT n TIMES
                        {
                            j <- 1
                            REPEAT n-1 TIMES
                            {
                                IF(aList[j] > aList[j + 1])
                                {
                                    temp <- aList[j]
                                    aList[j] <- aList[j + 1]
                                    aList[j + 1] <- temp
                                }
                                j <- j + 1
                            }
                        }
                        RETURN (aList)
                    }

                    a <- [1, 3, 2, 3, 4, 7, 2]
                    DISPLAY(bubbleSort(a))"#,
        "[1, 2, 2, 3, 3, 4, 7]",
    );
}

#[test]
fn test_merge_sort() {
    assert_output(
        r#"
            PROCEDURE merge(left, right) {
                result <- []
                leftIndex <- 1
                rightIndex <- 1

                REPEAT UNTIL(leftIndex > LENGTH(left) AND rightIndex > LENGTH(right)) {
                    IF(leftIndex > LENGTH(left)) {
                        APPEND(result, right[rightIndex])
                        rightIndex <- rightIndex + 1
                    } ELSE IF(rightIndex > LENGTH(right)) {
                        APPEND(result, left[leftIndex])
                        leftIndex <- leftIndex + 1
                    } ELSE IF(left[leftIndex] <= right[rightIndex]) {
                        APPEND(result, left[leftIndex])
                        leftIndex <- leftIndex + 1
                    } ELSE {
                        APPEND(result, right[rightIndex])
                        rightIndex <- rightIndex + 1
                    }
                }
                RETURN(result)
            }

            PROCEDURE mergeSort(arr) {
                IF(LENGTH(arr) <= 1) {
                    RETURN(arr)
                }

                mid <- LENGTH(arr) / 2
                left <- []
                right <- []

                i <- 1
                REPEAT mid TIMES {
                    APPEND(left, arr[i])
                    i <- i + 1
                }

                REPEAT LENGTH(arr) - mid TIMES {
                    APPEND(right, arr[i])
                    i <- i + 1
                }

                left <- mergeSort(left)
                right <- mergeSort(right)
                RETURN(merge(left, right))
            }

            arr <- [64, 34, 25, 12, 22, 11, 90]
            DISPLAY(mergeSort(arr))"#,
        "[11, 12, 22, 25, 34, 64, 90]",
    );
}

#[test]
fn test_quick_sort() {
    assert_output(
        r#"
            PROCEDURE partition(arr, low, high) {
    pivot <- arr[high]
    i <- low - 1

    j <- low
    REPEAT high - low TIMES {
        IF(arr[j] <= pivot) {
            i <- i + 1
            temp <- arr[i]
            arr[i] <- arr[j]
            arr[j] <- temp
        }
        j <- j + 1
    }

    temp <- arr[i + 1]
    arr[i + 1] <- arr[high]
    arr[high] <- temp
    RETURN([arr, i + 1])
}
PROCEDURE quickSort(arr, low, high) {
    IF(low < high) {
        partitionResult <- partition(arr, low, high)
        arr <- partitionResult[1]
        pi <- partitionResult[2]
        arr <- quickSort(arr, low, pi - 1)
        arr <- quickSort(arr, pi + 1, high)
    }
    RETURN(arr)
}

arr <- [64, 34, 25, 12, 22, 11, 90]
arr <- quickSort(arr, 1, LENGTH(arr))
DISPLAY(arr)"#,
        "[11, 12, 22, 25, 34, 64, 90]",
    );
}

#[test]
fn test_insertion_sort() {
    assert_output(
        r#"
            PROCEDURE insertionSort(arr) {
                    i <- 2
                    REPEAT LENGTH(arr) - 1 TIMES {
                        key <- arr[i]
                        j <- i - 1

                        IF(j >= 1 AND arr[j] > key) {
                            REPEAT UNTIL(j < 1 OR arr[j] <= key) {
                                arr[j + 1] <- arr[j]
                                j <- j - 1
                            }
                        }

                        arr[j + 1] <- key
                        i <- i + 1
                    }
                    RETURN(arr)
                }

                arr <- [64, 34, 25, 12, 22, 11, 90]
                DISPLAY(insertionSort(arr))"#,
        "[11, 12, 22, 25, 34, 64, 90]",
    );
}

#[test]
fn test_selection_sort() {
    assert_output(
        r#"
            PROCEDURE selectionSort(arr) {
                n <- LENGTH(arr)
                i <- 1

                REPEAT n - 1 TIMES {
                    minIdx <- i
                    j <- i + 1

                    REPEAT n - i TIMES {
                        IF(arr[j] < arr[minIdx]) {
                            minIdx <- j
                        }
                        j <- j + 1
                    }

                    IF(minIdx NOT= i) {
                        temp <- arr[minIdx]
                        arr[minIdx] <- arr[i]
                        arr[i] <- temp
                    }
                    i <- i + 1
                }
                RETURN(arr)
            }

            arr <- [64, 34, 25, 12, 22, 11, 90]
            DISPLAY(selectionSort(arr))"#,
        "[11, 12, 22, 25, 34, 64, 90]",
    );
}

#[test]
fn test_heap_sort() {
    assert_output(
        r#"
        PROCEDURE heapify(arr, n, i) {
            largest <- i
            left <- 2 * i
            right <- 2 * i + 1

            IF(left <= n AND arr[left] > arr[largest]) {
                largest <- left
            }

            IF(right <= n AND arr[right] > arr[largest]) {
                largest <- right
            }

            IF(largest NOT= i) {
                temp <- arr[i]
                arr[i] <- arr[largest]
                arr[largest] <- temp

                arr <- heapify(arr, n, largest)
            }
            RETURN(arr)
        }

        PROCEDURE heapSort(arr) {
            n <- LENGTH(arr)
            i <- n / 2
            REPEAT UNTIL(i < 1) {
                arr <- heapify(arr, n, i)
                i <- i - 1
            }

            i <- n
            REPEAT UNTIL(i < 1) {
                temp <- arr[1]
                arr[1] <- arr[i]
                arr[i] <- temp

                arr <- heapify(arr, i - 1, 1)
                i <- i - 1
            }
            RETURN(arr)
        }

        arr <- [12, 11, 13, 5, 6, 7]
        arr <- heapSort(arr)
        DISPLAY(arr)
        "#,
        "[5, 6, 7, 11, 12, 13]",
    );
}

#[test]
fn test_counting_sort() {
    assert_output(
        r#"
        PROCEDURE countingSort(arr, max_val) {
            count <- []
            i <- 1
            REPEAT (max_val + 1) TIMES {
                APPEND(count, 0)
                i <- i + 1
            }

            i <- 1
            REPEAT LENGTH(arr) TIMES {
                count[arr[i]] <- count[arr[i]] + 1
                i <- i + 1
            }

            i <- 2
            REPEAT max_val TIMES {
                count[i] <- count[i] + count[i - 1]
                i <- i + 1
            }

            output <- []
            i <- 1
            REPEAT LENGTH(arr) TIMES {
                APPEND(output, 0)
                i <- i + 1
            }

            i <- LENGTH(arr)
            REPEAT LENGTH(arr) TIMES {
                index <- count[arr[i]]
                output[index] <- arr[i]
                count[arr[i]] <- count[arr[i]] - 1
                i <- i - 1
            }
            RETURN(output)
        }

        arr <- [4, 2, 2, 8, 3, 3, 1]
        sorted <- countingSort(arr, 8)
        DISPLAY(sorted)
        "#,
        "[1, 2, 2, 3, 3, 4, 8]",
    );
}

#[test]
fn test_binary_search() {
    assert_output(
        r#"
            PROCEDURE binarySearch(arr, target) {
                left <- 1
                right <- LENGTH(arr)

                REPEAT UNTIL(left > right) {
                    mid <- (left + right) / 2

                    IF(arr[mid] = target) {
                        RETURN(mid)
                    } ELSE IF(arr[mid] < target) {
                        left <- mid + 1
                    } ELSE {
                        right <- mid - 1
                    }
                }
                RETURN(-1)
            }

            arr <- [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            DISPLAY(binarySearch(arr, 7))
            DISPLAY(binarySearch(arr, 11))"#,
        "7\n-1",
    );
}

#[test]
fn test_linear_search() {
    assert_output(
        r#"
            PROCEDURE linearSearch(arr, target) {
                i <- 1
                REPEAT LENGTH(arr) TIMES {
                    IF(arr[i] = target) {
                        RETURN(i)
                    }
                    i <- i + 1
                }
                RETURN(-1)
            }

            arr <- [64, 34, 25, 12, 22, 11, 90]
            DISPLAY(linearSearch(arr, 22))
            DISPLAY(linearSearch(arr, 100))"#,
        "5\n-1",
    );
}

#[test]
fn test_2d_linear_search() {
    assert_output(
        r#"
            PROCEDURE linearSearch2D(matrix, target) {
                rows <- LENGTH(matrix)
                columns <- LENGTH(matrix[1])

                i <- 1
                REPEAT rows TIMES {
                    j <- 1
                    REPEAT columns TIMES {
                        IF(matrix[i][j] = target) {
                            RETURN([i, j])
                        }
                        j <- j + 1
                    }
                    i <- i + 1
                }
                RETURN([-1, -1])
            }

            matrix <- [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            result <- linearSearch2D(matrix, 5)
            DISPLAY(result)
            result <- linearSearch2D(matrix, 10)
            DISPLAY(result)
            "#,
        "[2, 2]\n[-1, -1]",
    );
}

#[test]
fn test_kmp_string_matching() {
    assert_output(
        r#"
        PROCEDURE computeLPS(pattern) {
            lps <- []
            length <- 0
            i <- 1
            APPEND(lps, 0)

            REPEAT UNTIL(i >= LENGTH(pattern)) {
                IF(pattern[i + 1] = pattern[length + 1]) {
                    length <- length + 1
                    APPEND(lps, length)
                    i <- i + 1
                } ELSE {
                    IF(length NOT= 0) {
                        length <- lps[length]
                    } ELSE {
                        APPEND(lps, 0)
                        i <- i + 1
                    }
                }
            }
            RETURN(lps)
        }

        PROCEDURE kmpSearch(text, pattern) {
            lps <- computeLPS(pattern)
            i <- 1
            j <- 1
            positions <- []
            n <- LENGTH(text)
            m <- LENGTH(pattern)

            REPEAT UNTIL(i > n) {
                IF(pattern[j] = text[i]) {
                    i <- i + 1
                    j <- j + 1
                }

                IF(j > m) {
                    APPEND(positions, i - m)
                    j <- lps[j - 1] + 1
                } ELSE IF(i <= n AND pattern[j] NOT= text[i]) {
                    IF(j NOT= 1) {
                        j <- lps[j - 1] + 1
                    } ELSE {
                        i <- i + 1
                    }
                }
            }
            RETURN(positions)
        }

        text <- "ABABDABACDABABCABAB"
        pattern <- "ABABCABAB"
        positions <- kmpSearch(text, pattern)
        DISPLAY(positions)
        "#,
        "[11]",
    );
}

#[test]
fn test_gcd_recursive() {
    assert_output(
        r#"
            PROCEDURE gcd(a, b) {
                IF(b = 0) {
                    RETURN(a)
                }
                RETURN(gcd(b, a MOD b))
            }

            DISPLAY(gcd(48, 18))
            DISPLAY(gcd(54, 24))
            DISPLAY(gcd(17, 5))"#,
        "6\n6\n1",
    );
}

#[test]
fn test_calc_functions() {
    assert_output(
        r#"
            PROCEDURE DERIVATIVE(coefficients, exponents)
            {
                result_coeffs <- []
                result_exps <- []
                i <- 1

                REPEAT LENGTH(coefficients) TIMES
                {
                    IF (exponents[i] NOT= 0)
                    {
                        new_coeff <- coefficients[i] * exponents[i]
                        new_exp <- exponents[i] - 1
                        APPEND(result_coeffs, new_coeff)
                        APPEND(result_exps, new_exp)
                    }
                    i <- i + 1
                }
                RETURN([result_coeffs, result_exps])
            }

            PROCEDURE ANTIDERIVATIVE(coefficients, exponents)
            {
                result_coeffs <- []
                result_exps <- []
                i <- 1

                REPEAT LENGTH(coefficients) TIMES
                {
                    new_exp <- exponents[i] + 1
                    new_coeff <- coefficients[i] / new_exp
                    APPEND(result_coeffs, new_coeff)
                    APPEND(result_exps, new_exp)
                    i <- i + 1
                }
                APPEND(result_coeffs, 0)
                APPEND(result_exps, 0)
                RETURN([result_coeffs, result_exps])
            }

            coeffs <- [3, 2, 1]
            exps <- [2, 1, 0]

            deriv <- DERIVATIVE(coeffs, exps)
            DISPLAY("Derivative coefficients: " + TOSTRING(deriv[1]))
            DISPLAY("Derivative exponents: " + TOSTRING(deriv[2]))

            coeffs2 <- deriv[1]
            exps2 <- deriv[2]

            antideriv <- ANTIDERIVATIVE(coeffs2, exps2)
            DISPLAY("Antiderivative coefficients: " + TOSTRING(antideriv[1]))
            DISPLAY("Antiderivative exponents: " + TOSTRING(antideriv[2]))
        "#,
        "Derivative coefficients: [6, 2]\nDerivative exponents: [1, 0]\nAntiderivative coefficients: [3, 2, 0]\nAntiderivative exponents: [2, 1, 0]",
    );
}

#[test]
fn test_min_max_functions() {
    assert_output(
        r#"
            PROCEDURE test_min_max() {
                a <- MIN(5, 10)
                b <- MIN(10, 5)
                c <- MAX(5, 10)
                d <- MAX(10, 5)
                DISPLAY(a)
                DISPLAY(b)
                DISPLAY(c)
                DISPLAY(d)
            }

            test_min_max()
            "#,
        "5\n5\n10\n10",
    );
}

#[test]
fn test_fibonacci_seq() {
    assert_output(
        r#"
        PROCEDURE fibonacci(n)
        {
            a <- 0
            b <- 1
            result <- [a, b]

            REPEAT (n-2) TIMES
            {
                temp <- a + b
                APPEND(result, temp)
                a <- b
                b <- temp
            }

            RETURN(result)
        }

        n <- 10
        fibSequence <- fibonacci(n)
        DISPLAY(fibSequence)
        "#,
        "[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]",
    );

    assert_output(
        r#"
            PROCEDURE fibonacci(n)
            {
                IF(n <= 0)
                {
                    RETURN(0)
                }
                IF(n = 1)
                {
                    RETURN(1)
                }
                RETURN(fibonacci(n - 1) + fibonacci(n - 2))
            }
            DISPLAY(fibonacci(6))"#,
        "8",
    );
}

#[test]
fn test_ml_algorithms() {
    assert_output(
        r#"
            PROCEDURE calculateDistance(point1, point2)
            {
                sum <- 0
                i <- 1
                REPEAT LENGTH(point1) TIMES
                {
                    diff <- point1[i] - point2[i]
                    sum <- sum + POW(diff, 2)
                    i <- i + 1
                }
                RETURN(SQRT(sum))
            }

            PROCEDURE findKNearest(trainingData, trainingLabels, testPoint, k)
            {
                distances <- []
                labels <- []

                i <- 1
                REPEAT LENGTH(trainingData) TIMES
                {
                    distance <- calculateDistance(testPoint, trainingData[i])
                    APPEND(distances, distance)
                    APPEND(labels, trainingLabels[i])
                    i <- i + 1
                }

                sortedIndices <- []
                i <- 1
                REPEAT LENGTH(distances) TIMES
                {
                    minIndex <- 1
                    j <- 1
                    REPEAT LENGTH(distances) TIMES
                    {
                        IF(distances[j] < distances[minIndex])
                        {
                            minIndex <- j
                        }
                        j <- j + 1
                    }
                    APPEND(sortedIndices, minIndex)
                    distances[minIndex] <- 999999999
                    i <- i + 1
                }

                kNearest <- []
                i <- 1
                REPEAT k TIMES
                {
                    APPEND(kNearest, labels[sortedIndices[i]])
                    i <- i + 1
                }

                RETURN(kNearest)
            }

            PROCEDURE getMajorityVote(labels)
            {
                counts <- []
                uniqueLabels <- []

                FOR EACH label IN labels
                {
                    found <- FALSE
                    i <- 1
                    REPEAT LENGTH(uniqueLabels) TIMES
                    {
                        IF(label = uniqueLabels[i])
                        {
                            counts[i] <- counts[i] + 1
                            found <- TRUE
                        }
                        i <- i + 1
                    }
                    IF(NOT found)
                    {
                        APPEND(uniqueLabels, label)
                        APPEND(counts, 1)
                    }
                }

                maxCount <- 0
                maxLabel <- uniqueLabels[1]
                i <- 1
                REPEAT LENGTH(counts) TIMES
                {
                    IF(counts[i] > maxCount)
                    {
                        maxCount <- counts[i]
                        maxLabel <- uniqueLabels[i]
                    }
                    i <- i + 1
                }

                RETURN(maxLabel)
            }

            PROCEDURE knn(trainingData, trainingLabels, testPoint, k)
            {
                nearestLabels <- findKNearest(trainingData, trainingLabels, testPoint, k)
                prediction <- getMajorityVote(nearestLabels)
                RETURN(prediction)
            }

            trainingData <- [[1,2], [2,3], [3,1], [6,5], [7,8], [8,7]]
            trainingLabels <- [1, 1, 1, 2, 2, 2]
            testPoint <- [5,5]
            k <- 3

            prediction <- knn(trainingData, trainingLabels, testPoint, k)
            DISPLAY(prediction)
            "#,
        "2",
    );

    assert_output(
        r#"
        PROCEDURE linearRegression(x, y) {
            n <- LENGTH(x)
            sumX <- 0
            sumY <- 0
            sumXY <- 0
            sumXSquare <- 0

            i <- 1
            REPEAT n TIMES {
                sumX <- sumX + x[i]
                sumY <- sumY + y[i]
                sumXY <- sumXY + x[i] * y[i]
                sumXSquare <- sumXSquare + x[i] * x[i]
                i <- i + 1
            }

            slope <- (n * sumXY - sumX * sumY) / (n * sumXSquare - sumX * sumX)
            intercept <- (sumY - slope * sumX) / n

            RETURN([slope, intercept])
        }

        PROCEDURE predict(x, coefficients) {
            slope <- coefficients[1]
            intercept <- coefficients[2]
            RETURN(slope * x + intercept)
        }

        x <- [1, 2, 3, 4, 5]
        y <- [2, 4, 6, 8, 10]

        coefficients <- linearRegression(x, y)
        prediction <- predict(6, coefficients)
        DISPLAY(FLOOR(prediction))
        "#,
        "12",
    );
}
