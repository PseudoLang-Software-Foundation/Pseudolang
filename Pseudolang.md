# PseudoLang Guide

## Assignment, Display, Input, Casting

`a <- b`

Evaluates b and then assigns a copy of the result to the variable a

`DISPLAY(a)` or `DISPLAY()`

Prints the value of a followed by a newline. When called with no arguments, prints an empty line.

`DISPLAYINLINE(a)`

Prints the value of a, without a new line character.

`INPUT()`

Accepts a value from the user (command line) and returns the input value.

`TOSTRING(a)`
Converts an integer/float data type to a string.

`TONUM(a)`
Converts a string to an integer or a float.

## Mathematical Procedures

`a + b`

`a - b`

`a * b`

`a / b`

Integer division that truncates toward zero. For example:

- `5 / 2` evaluates to `2`
- `-5 / 2` evaluates to `-2`
- `19 / 4` evaluates to `4`

When operating on two integers, the result will always be an integer with the fractional part discarded (truncated toward zero). When either operand is a float, the result is a float with standard floating-point division.

`a MOD b`

The arithmetic operators +, -, *, /, and MOD are used to perform arithmetic on a and b.
MOD accepts the same operand types as the other arithmetic operators: two integers
give an integer remainder, and any combination involving a float (`Float MOD Float`,
`Int MOD Float`, `Float MOD Int`) gives a floating-point remainder. A modulus of zero
is an error.

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

`LOG(x)` / `NLOG(x)`

Returns the natural logarithm (ln) of x.

`LOGTEN(x)`

Returns the base-10 logarithm of x.

`LOGTWO(x)`

Returns the base-2 logarithm of x.

`GCD(a, b)`

Returns the greatest common divisor of a and b.

`FACTORIAL(x)`

Returns the factorial of x. Works for any non-negative integer (arbitrary precision).

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

Both AND and OR require Boolean operands on *both* sides; a non-Boolean operand is a
runtime error, for example `1 AND TRUE`. They still short-circuit, so `FALSE AND b`
and `TRUE OR b` never evaluate `b` at all.

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

The code in the block of statements is executed first, then the Boolean expression a is evaluated. If a is false, the block executes again. This repeats until a evaluates to true. The body always executes at least once (do-while semantics).

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

`LENGTH` also accepts a string, where it evaluates to the number of **characters**
in that string. For text outside ASCII this is not the same as the number of bytes:
`LENGTH("héllo")` is 5, and `LENGTH("日本語")` is 3.

`SORT(aList)`

Returns a new list that is a sorted version of `aList`. The sorting is done in ascending order. Supports lists of integers, floats, mixed numeric types, and strings (sorted lexicographically).

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

## Dictionary operations

`aDict <- {"name": "Bob", "age": 30}`

Creates a new dictionary mapping each key to its value and assigns it to aDict. A dictionary literal is only recognised in expression position; a `{` at the start of a statement always opens a block, never a dictionary.

`aDict <- {}` or `aDict <- DICTIONARY()`

Both create a new empty dictionary. `DICTIONARY()` takes no arguments and is useful where a `{}` would be ambiguous, such as at the start of a statement.

```psl
aDict <- {
    "a": 1,
    "b": 2
}
```

A dictionary literal may span multiple lines. Newlines are allowed before and after keys, colons, values and commas.

`aDict[k]`

Evaluates to the value stored under key k. Reading a key that is not present is an error: `Key not found: k`.

`aDict[k] <- b`

Assigns b to key k. Unlike a list index, a key that does not exist yet is created rather than being an error; an existing key is overwritten in place.

Keys must be strings, integers or booleans. Using a float, NULL, NAN, a list or a dictionary as a key raises `Dictionary keys must be strings, integers, or booleans`. Keys of different types never collide, so `1` and `"1"` are two distinct keys — note that because keys display unquoted, two such keys look alike when the whole dictionary is displayed.

```psl
aDict <- {"s": 1, 2: "two", TRUE: "yes"}
DISPLAY(aDict[2]) COMMENT Displays two
```

Dictionaries preserve insertion order. Overwriting an existing key keeps that key in its original position, while a brand new key is appended at the end. `DISPLAY`, `KEYS`, `VALUES` and `FOR EACH` all report that same order.

```psl
aDict <- {"a": 1, "b": 2}
aDict["a"] <- 99 COMMENT "a" stays first
aDict["c"] <- 3  COMMENT "c" is appended
DISPLAY(aDict)   COMMENT Displays {a: 99, b: 2, c: 3}
```

Displaying a dictionary produces `{key: value, key: value}`, with scalars written unquoted exactly the way lists render as `[apple, banana]`. An empty dictionary displays as `{}`.

`aDict <- bDict`

Assigns a copy of the dictionary bDict to aDict. As with lists, dictionaries are copied on assignment and when passed to a procedure, so changes made through one name are not visible through the other.

`aDict = bDict` or `aDict NOT= bDict`

Two dictionaries are equal when they hold the same set of keys with equal values, regardless of insertion order. The comparison is deep, so nested lists and dictionaries are compared structurally. The ordering operators `<`, `>`, `<=` and `>=` are not supported for dictionaries.

`aDict + bDict`

The `+` operator merges two dictionaries into a new dictionary. Keys from aDict keep their positions, keys only in bDict are appended, and where both sides define a key the value from bDict wins.

Example:

```psl
a <- {"x": 1, "y": 2}
b <- {"y": 99, "z": 3}
DISPLAY(a + b)
```

This will display {x: 1, y: 99, z: 3}.

```psl
FOR EACH key IN aDict
{
 <statement(s)>
}
```

Iterating a dictionary assigns each of its keys to the loop variable, in insertion order. Use `aDict[key]` inside the body to reach the matching value.

`KEYS(aDict)`

Returns a list of the dictionary's keys in insertion order.

`VALUES(aDict)`

Returns a list of the dictionary's values in insertion order, aligned element by element with `KEYS(aDict)`.

`HASKEY(aDict, k)`

Returns TRUE if the dictionary contains key k, FALSE otherwise. A k that could never be a key, such as a float or NULL, is simply reported as absent, so `HASKEY` is safe to use as a guard for any value.

`GETKEY(aDict, k)` or `GETKEY(aDict, k, default)`

Returns the value stored under key k. With a third argument, a missing key evaluates to default instead of raising an error; without one, a missing key raises `Key not found: k`.

`SETKEY(aDict, k, b)`

Stores b under key k in the dictionary variable aDict, creating the key if needed, and returns b. Like `APPEND`, the first argument must be a variable rather than a literal because the dictionary is modified in place.

`REMOVEKEY(aDict, k)`

Removes key k from the dictionary variable aDict and returns the value that was stored there. Removing a key that is not present raises `Key not found: k`.

`REMOVE(aDict, k)`

`REMOVE` also accepts a dictionary, where it behaves exactly like `REMOVEKEY` and removes by key rather than by index.

`LENGTH(aDict)`

Evaluates to the number of key-value pairs in the dictionary.

`DICTIONARY`, `KEYS`, `VALUES`, `HASKEY`, `GETKEY`, `SETKEY` and `REMOVEKEY` are built-in names. Like every other built-in, they are resolved before user-defined procedures, so a procedure declared with one of those names is never called.

Dictionaries interpolate into formatted strings the same way lists do, so `f"{aDict}"` renders the `{key: value}` form. Writing a dictionary *literal* inside a formatted string works but is fragile: the lexer finds the end of an interpolation slot by counting braces, so a slot such as `f"{ {"a": "}"} }"` ends early on the `}` inside the string value and fails to parse. Build the dictionary in a variable first when a value may contain a brace.

```psl
PROCEDURE procName(a, b)
{
 <statement(s)>
}
```

Defines procName as a procedure that takes zero or more arguments. The procedure contains statements. The procedure procName can be called using the following notation, where arg1 is assigned to parameter1, arg2 is assigned to parameter2 and so on.
`procName(arg1, arg2)`

`RETURN (a)` or `RETURN` or `RETURN ()`

Returns the flow of control to the point where the procedure was called and optionally returns a value. When a procedure executes a value-less return or reaches its end without an explicit return value, displaying the procedure's result will show nothing.

Procedures have their own scope. Variables assigned inside a procedure are local to that procedure and do not modify variables of the same name in the calling scope. Parameters shadow any outer variables with the same name. Procedures defined at the top level are visible to all other procedures (including mutually recursive calls).

```psl
x <- 10
PROCEDURE setX()
{
    x <- 99
    DISPLAY(x) COMMENT Displays 99
}
setX()
DISPLAY(x) COMMENT Displays 10 (outer x unchanged)
```

`SUBSTRING("abcd", start, end)`
Returns a string of characters from index `start` to index `end` of the given string
(both inclusive, 1-based). `start` and `end` are character positions, not byte
offsets, so `SUBSTRING("héllo", 1, 2)` is `"hé"`. Indices outside the string, or an
`end` before the `start`, are a runtime error.

`CONCAT("ab", "cd")`
Returns a single string with the two given strings combined

`CONTAINS(string, text)`

Returns TRUE if the string contains the given text, FALSE otherwise.

`FIND(string, text)`

Returns the index position of the first occurrence of text in string (1-based indexing). Returns -1 if text is not found.
The position is a character position, so it can be handed straight back to `string[i]`
or `SUBSTRING` even when the string contains non-ASCII text.

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

Integer (arbitrary precision -- integers have unlimited size)

`0.1`

Float (64 bit)

`"a"`

String. Strings are UTF-8 text and are indexed by character, not by byte: `LENGTH`,
`s[i]`, `SUBSTRING` and `FIND` all agree on the same 1-based character positions, so
a position produced by one can be handed to another regardless of the alphabet used.

`TRUE` or `FALSE`

Boolean

`{"a": 1}`

Dictionary (insertion-ordered key-value pairs; keys may be strings, integers or booleans)

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
CLASS className
{
  ...procs
}
```

**Planned feature (not yet implemented).** Class declarations are parsed but instantiation, method dispatch, and field access are not yet supported. Using CLASS will produce a runtime error.

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

## Command-Line Arguments

PseudoLang programs can access CLI arguments passed after the `.psl` file path:

```
fpli run program.psl --verbose -n 5 output.txt
```

Flags placed before the `.psl` file (like `--debug`) are consumed by `fpli` itself. Everything after the file is forwarded to the program.

### Built-in Variables

| Variable | Type | Description |
|----------|------|-------------|
| `ARGS` | List | All raw arguments as strings |
| `ARGCOUNT` | Integer | Number of arguments |
| `POSITIONALS` | List | Non-flag arguments only |

```psl
DISPLAY(ARGS)           COMMENT returns ["--verbose", "-n", "5", "output.txt"]
DISPLAY(ARGCOUNT)       COMMENT returns 4
DISPLAY(POSITIONALS)    COMMENT returns ["output.txt"]
```

### Built-in Functions

`HASARG(name)` — Returns `TRUE` if a flag exists. Leading dashes in the query are stripped automatically.

```psl
HASARG("verbose")   COMMENT matches --verbose
HASARG("--verbose") COMMENT also matches --verbose
HASARG("n")         COMMENT matches -n
```

`GETARG(name)` — Returns the value of a flag. Errors if the flag is not found.

`GETARG(name, default)` — Returns the value or the default if the flag is missing.

```psl
DISPLAY(GETARG("n"))                COMMENT returns "5"
DISPLAY(GETARG("missing", "N/A"))   COMMENT returns "N/A"
DISPLAY(GETARG("verbose"))          COMMENT returns "true" (boolean flags)
```

### Parsing Rules

- `--key value` or `-k value`: next non-flag argument is captured as the value
- `--flag` or `-f` (followed by another flag or end): treated as a boolean flag with value `"true"`
- Anything not starting with `-`: added to `POSITIONALS`

## Limitations

Since a lot of the syntax is text like COMMENT or TRUE, you may not set variables as such, and the interpreter will try to raise an error if it occurs.

Expressions and blocks may not nest more than 128 levels deep. Each nested `(`, `[`,
call argument or `{` block counts as one level; exceeding the limit is reported as
`Maximum nesting depth exceeded`. This is far more nesting than a readable program
needs, and it keeps pathological input from exhausting the interpreter's stack.
