# PseudoLang Guide

## Assignment, Display, Input, Casting

`a <- b`

Evaluates b and then assigns a copy of the result to the variable a

`DISPLAY(a)`

Prints the value of a.

`DISPLAYINLINE(a)`

Prints the value of a, without a new line character.

`INPUT()`

Accepts a value from the user (command line) and returns the input value.

`TOSTRING(a)`
Converts an integer/float data type to a string.

`TONUM(a)`
Covnerts a string to an integer or a float.

## Mathematical Procedures

`a + b`

`a - b`

`a * b`

`a / b`

Integer division that rounds down (floor division). For example:

- `5 / 2` evaluates to `2`
- `-5 / 2` evaluates to `-2`
- `19 / 4` evaluates to `4`

When operating on two integers, the result will always be an integer, rounded down to the nearest whole number.

`a MOD b`

The arithmetic operators +, -, *, /, and MOD are used to perform arithmetic on a and b.

`RANDOM(a, b)`

Generates and returns a random integer from a to b, including a and b. Each result is equally likely to occur.

`ABS(x)`

Returns the absolute value of x.

`CEIL(x)`

Returns the smallest integer greater than or equal to x.

`FLOOR(x)`

Returns the largest integer less than or equal to x.

`POW(x, y)`

Returns x raised to the power y.

`SQRT(x)`

Returns the square root of x.

`SIN(x)`

Returns the sine of x (x in radians).

`COS(x)`

Returns the cosine of x (x in radians).

`TAN(x)`

Returns the tangent of x (x in radians).

`ASIN(x)`

Returns the arc sine of x, in radians.

`ACOS(x)`

Returns the arc cosine of x, in radians.

`ATAN(x)`

Returns the arc tangent of x, in radians.

`EXP(x)`

Returns e raised to the power x.

`LOG(x)`

Returns the natural logarithm of x.

`LOGTEN(x)`

Returns the base-10 logarithm of x.

`LOGTWO(x)`

Returns the base-2 logarithm of x.

`GCD(a, b)`

Returns the greatest common divisor of a and b.

`FACTORIAL(x)`

Returns the factorial of x.

`DEGREES(x)`

Converts angle x from radians to degrees.

`RADIANS(x)`

Converts angle x from degrees to radians.

`MIN(a, b)`

Returns the smaller value of a and b.

`MAX(a, b)`

Returns the larger value of a and b.

`HYPOT(x, y)`

Returns the Euclidean norm, sqrt(x*x + y*y).

`ROUND(x)`

Returns the value of x rounded to the nearest integer.

## Relational and Boolean Operators

`a = b`

`a NOT= b`

`a > b`

`a < b`

`a >= b`

`a <= b`

The relational operators =, NOT=, >, <, >=, and <= are used to test the relationship between two variables, expressions, or values. A comparison using relational operators evaluates to a Boolean value.

`a AND b`

Evaluates to true if both a and b are true; otherwise evaluates to false.

`a OR b`

Evaluates to true if a is true or if b is true or if both a and b are true; otherwise evaluates to false.

`NOT a`

Evaluates to true if a is false; otherwise evaluates to false.

## Selection

```psl
IF(a)
{
 <statement(s)>
}
```

The code in block of statements is executed if the Boolean expression a evaluates to true; no action is taken if condition evaluates to false.

```psl
IF(a)
{
 <first statement(s)>
}
ELSE
{
 <second statement(s)>
}
```

The code in first block of statements is executed if the Boolean expression a evaluates to true; otherwise the code in second block of statements is executed.

```psl
REPEAT n TIMES
{
 <statement(s)>
}
```

The code inside is executed n times.

```psl
REPEAT UNTIL(a)
{
 <statement(s)>
}
```

The code in block of statements is repeated until the Boolean expression a evaluates to true.

## List operations

`aList <- [1, 2, 3]`

Creates a new list that contains the values 1, 2, 3 at index 1, 2, 3. Respectively and assigns it to aList, this also works with empty lists.

`aList <- bList`

Assigns a copy of the list bList to the list aList.

`aList[i]`

Accesses the element of aList at index i. The first element of aList is at index 1 and is accessed using the notation aList[1].

`b <- aList[i]` or `aList[i] <- b`

Assigns the value of aList[i] to the variable b, or assigns the value of aList[i] to the variable b.

`aList[b] <- aList[c]`

Assigns the value of aList[c] to aList[b].

`INSERT(aList, i, b)`

Any values in aList at indices greater than or equal to i are shifted one position to the right. The length of the list is increased by 1, and value b is placed at index i in aList.

`APPEND(aList, b)`

The length of aList is increased by 1, and value b is placed at
the end of aList.

`REMOVE(aList, i)`

Removes the item at index i in aList and shifts to the left any values at indices greater than i. The length of aList is decreased by 1.

`LENGTH(aList)`

Evaluates to the number of elements in aList (1 through length).

`SORT(aList)`

Returns a new list that is a sorted version of `aList` (must be an array of integers). The sorting is done in ascending order.

`RANGE(start (optional), end)`

Creates a new list containing integers from start (1 by default) to end inclusive.

`aList + bList`

The `+` operator can be used to concatenate two lists. This creates a new list containing all the elements of `aList` followed by all the elements of `bList`.

Example:

```psl
a <- [1, 2, 3]
b <- [4, 5, 6]
c <- a + b
DISPLAY(c)
```

This will display [1, 2, 3, 4, 5, 6].

```psl
FOR EACH item IN aList
{
 <statement(s)>
}
```

The variable item is assigned the value of each element of aList sequentially, in order, from the first element to the last element. The statements are executed once for each assignment of item.

```psl
matrix <- [[1, 2, 3], [4, 5, 6]]
DISPLAY(matrix[1][1]) COMMENT Should be 1
```

Multi-dimensional arrays (also called matrices or N-D arrays) can be created and manipulated using nested lists. All list operations (LENGTH, APPEND, REMOVE, etc.) can be applied to any dimension of the array.

```psl
PROCEDURE procName(a, b)
{
 <statement(s)>
}
```

Defines procName as a procedure that takes zero or more arguments. The procedure contains statements. The procedure procName can be called using the following notation, where arg1 is assigned to parameter1, arg2 is assigned to parameter2 and so on.
`procName(arg1, arg2)`

`RETURN (a)` or `RETURN` or `RETURN ()`

Returns the flow of control to the point where the procedure was called without returning a value. When a procedure executes a value-less return or reaches its end without an explicit return value, displaying the procedure's result will show nothing.

`SUBSTRING("abcd", start, end)`
Returns a string of characters from index `start` to index `end` of the given string

`CONCAT("ab", "cd")`
Returns a single string with the two given strings combined

`CONTAINS(string, text)`

Returns TRUE if the string contains the given text, FALSE otherwise.

`FIND(string, text)`

Returns the index position of the first occurrence of text in string (1-based indexing). Returns -1 if text is not found.

`SPLIT(string, delimiter)`

Splits a string into parts based on the given delimiter and returns a list of strings.

`TRIM(string)`

Removes leading and trailing whitespace from a string.

`REPLACE(string, from, to)`

Returns a new string with all occurrences of `from` replaced with `to`.

`UPPERCASE(string)`

Converts all characters in the string to uppercase.

`LOWERCASE(string)`

Converts all characters in the string to lowercase.

`STARTSWITH(fullstring, substring)`

Returns TRUE if the fullstring starts with the given substring, FALSE otherwise.

`ENDSWITH(fullstring, substring)`

Returns TRUE if the fullstring ends with the given substring, FALSE otherwise.

## Data Types

`1`

Integer (64 bit)

`0.1`

Float (64 bit)

`"a"`

String (64 bit)

`TRUE` or `FALSE`

Boolean

`NULL`

A special value representing the absence of a value.

`NAN`

A special numeric value representing an undefined or unrepresentable value. Any arithmetic operation involving NAN results in NAN. Comparing NAN with any value (including another NAN) returns false, except for NAN NOT= NAN which returns true.

## Methods

```psl
COMMENT a
```

```psl
COMMENTBLOCK
a
b
COMMENTBLOCK
```

A comment (multi-line or single-line), anything on the line after this or in between does not affect the code.

`IMPORT a`

Imports a library (including functions & variables defined in that file) from a file.

```psl
CLASS className()
{
  ...procs
}
```

Creates a class object

`r"a"`

Creates a raw string.

`f"a{b}"`

Creates a formatted string, the string value of the variable is added to the string.

`SLEEP(x)`

Pauses program execution for x seconds. x can be an integer or a floating-point number.

`TIMESTAMP()`

Returns the current Unix timestamp (seconds since January 1, 1970 UTC).

`TIMESTAMP(datetime)`

Converts a datetime string in format "YYYY-MM-DD HH:MM:SS.ffffff" to Unix timestamp.

`TIME(timestamp)`

Converts a Unix timestamp to a datetime string in format "YYYY-MM-DD HH:MM:SS.ffffff" in local time.

`TIMEZONE(timestamp, timezone)`  
Converts a Unix timestamp to a datetime string in the specified timezone.
Example timezones: "America/New_York", "Europe/London", "Asia/Tokyo"

`TIMEZONES()`
Returns a list of all available timezone names.

```psl
TRY {
    DISPLAY("Before error")
    x <- 1 / 0 COMMENT Causes error
    DISPLAY("After error") COMMENT Never executes
} CATCH (err) {
    DISPLAY("Caught error: " + err)  COMMENT Will display "Caught error: Division by zero"
}
```

The try-catch statement allows you to handle errors that might occur during program execution. Any statements inside the try block that cause an error will stop execution of that block and transfer control to the catch block. The error message is stored in the variable specified in parentheses after catch and can be used inside the catch block.

```psl
expression <- "x* (x+1)*(x+2)"
x <- 3
DISPLAY(EVAL(expression))
```

EVAL takes in a string expression, that will return the evaluated response as if it were executed in the program.

`EXIT()`

Terminates program execution immediately.

## Limitations

Since a lot of the syntax is text like COMMENT or TRUE, you may not set variables as such, and the interpreter will try to raise an error if it occurs.
