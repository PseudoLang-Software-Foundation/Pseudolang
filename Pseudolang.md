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

`anyExpression[i]`

`[i]` may follow any expression, not only a variable name, and several may be
chained.

```psl
DISPLAY(SPLIT("a,b,c", ",")[2])   COMMENT returns b
DISPLAY(LISTDIR(".")[1])          COMMENT the first entry in this directory
DISPLAY("hello"[1])               COMMENT returns h
DISPLAY([[1, 2], [3, 4]][2][1])   COMMENT returns 3
DISPLAY({"k": [9, 8]}["k"][2])    COMMENT returns 8
```

The left-hand side of an assignment must still start from a variable: `aList[i] <- b`
is valid, `f()[i] <- b` is not. An indexed value cannot be called either: there are no
callable values, so `handlers[1](x)` is an error rather than a call.

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

A special value representing the absence of a value. `WHICH`, `PROCESSINFO`,
`HOSTNAME` and `SCRIPTPATH` return it when there is nothing to report.

```psl
IF WHICH("git") = NULL
{
    DISPLAY("git is not installed")
}
```

`NULL` equals `NULL` and is unequal to every other value. `<` and `>` against
`NULL` are always false.

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
See [Libraries and Multiple Files](#libraries-and-multiple-files) for how the file is
found and for the once-only rule.

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

```psl
x <- """line one
line two"""
```

Creates a multiline string. Newlines inside the triple quotes are kept, and `\`
escapes are not processed.

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

Assignments inside either block persist afterwards, as in `IF`, `FOR EACH` and
`REPEAT`:

```psl
TRY
{
    config <- READFILE("config.txt")
} CATCH (err)
{
    config <- "defaults"
}
DISPLAY(config)             COMMENT "defaults" when the file is missing
```

The error variable is the exception: it exists only inside the catch block, and a
variable of the same name from outside is restored afterwards. Only a `PROCEDURE`
body gets a private scope.

```psl
expression <- "x* (x+1)*(x+2)"
x <- 3
DISPLAY(EVAL(expression))
```

EVAL takes in a string expression, that will return the evaluated response as if it were executed in the program.

`EXIT()`

Terminates program execution immediately.

## File IO

PseudoLang can read and write text files on disk. Paths are used exactly as given, so
a relative path resolves against the directory `fpli` was run from, not the directory
the `.psl` file lives in.

Every one of these functions raises an ordinary runtime error when the operation
fails, and the message names both the function and the path. That means a failure can
be handled with `TRY`/`CATCH` like any other error:

```psl
TRY
{
    config <- READFILE("config.txt")
} CATCH (err)
{
    DISPLAY("Using defaults: " + err)
    config <- ""
}
```

### Reading

`READFILE(path)` — Returns the whole file as a single string. Errors if the file is
missing, is a directory, or is not valid UTF-8 text.

`READLINES(path)` — Returns a list of the file's lines as strings. Line terminators
are stripped, `\r\n` and `\n` are both recognised, and a file ending in a newline does
not produce a trailing empty entry. An empty file gives an empty list.

```psl
total <- 0
FOR EACH line IN READLINES("scores.txt")
{
    total <- total + TONUM(line)
}
DISPLAY(total)
```

### Writing

`WRITEFILE(path, text)` — Creates the file if it does not exist and **replaces** its
contents otherwise. `text` must be a string; use `TOSTRING` to write a number.

`APPENDFILE(path, text)` — Adds `text` to the end of the file, creating it first if
needed. Nothing is inserted between appends, so write your own `"\n"` for line-based
output.

```psl
APPENDFILE("log.txt", "started\n")
APPENDFILE("log.txt", f"result: {TOSTRING(x)}\n")
```

### Inspecting and removing

`FILEEXISTS(path)` — Returns `TRUE` if something exists at `path` (a file or a
directory). This is the one file function that never errors.

`FILESIZE(path)` — Returns the size in **bytes** as an integer. For non-ASCII text
this differs from `LENGTH(READFILE(path))`, which counts characters.

`FILEMTIME(path)` — When the file was last modified, as Unix seconds.

```psl
IF FILEMTIME("input.csv") > FILEMTIME("report.txt")
{
    DISPLAY("the report is out of date")
}
DISPLAY(TIME(FILEMTIME("input.csv")))
```

`DELETEFILE(path)` — Removes a file. A directory is refused, with an error naming
the two functions below.

`DELETEDIR(path)` — Removes an empty directory. A directory that still holds
anything is refused.

`DELETETREE(path)` — Removes a directory and everything inside it. A plain file is
refused.

```psl
MAKEDIR("build/artifacts")
COMMENT ... work ...
DELETETREE("build")             COMMENT the whole tree
MAKEDIR("empty")
DELETEDIR("empty")              COMMENT only if it is empty
```

`LISTDIR(path)` — Returns the names of the entries in a directory, sorted
alphabetically. The entries are bare names, not full paths.

`MAKEDIR(path)` — Creates a directory, including any missing parent directories.
Succeeds if the directory already exists.

```psl
MAKEDIR("output/reports")
WRITEFILE("output/reports/summary.txt", "done")
names <- LISTDIR("output/reports")
DISPLAY(names)              COMMENT returns [summary.txt]
```

### Moving and copying

`RENAME(from, to)` — Moves or renames a file, replacing the destination if it
already exists.

`ISFILE(path)` and `ISDIR(path)` — Ask specifically whether a path is a file or a
directory. `FILEEXISTS` answers for either.

`COPYFILE(from, to)` — Copies a file, overwriting the destination. Returns the
number of bytes copied.

```psl
IF ISFILE("draft.txt")
{
    n <- COPYFILE("draft.txt", "backup.txt")
    DISPLAY(f"backed up {TOSTRING(n)} bytes")
    RENAME("draft.txt", "final.txt")
}
```

`WRITEFILE`, `APPENDFILE`, `DELETEFILE`, `DELETEDIR`, `DELETETREE`, `MAKEDIR` and
`RENAME` return no value.

### Availability

File IO needs a real filesystem. It works in the native `fpli` binary and under WASI,
but in the browser-embedded WebAssembly build (including the web IDE) every file
function raises an error explaining that the browser sandbox has no filesystem.

## Paths and the Working Directory

These are string operations on paths -- correct for Windows separators and POSIX
ones alike, because they use the host's own path rules rather than assuming `/`.
Except for `REALPATH`, none of them touches the filesystem, so they work on paths
that do not exist yet.

`JOINPATH(a, b, ...)` — Joins any number of segments with the host's separator.
Prefer this to `CONCAT` with a literal `"/"`.

`BASENAME(path)` — The final component. `"a/b/c.txt"` gives `"c.txt"`.

`DIRNAME(path)` — Everything before the final component. `"a/b/c.txt"` gives
`"a/b"`, and a bare filename gives `""`.

`EXTENSION(path)` — The extension without its dot, or `""` if there is none.
`"a/b.tar.gz"` gives `"gz"`.

`ABSPATH(path)` — Resolves a relative path against the working directory. Does not
require the path to exist and does not follow symlinks or collapse `..`.

`REALPATH(path)` — Resolves a path all the way to a real location, following
symlinks. The path must exist.

```psl
p <- JOINPATH("data", "raw", "input.csv")
DISPLAY(BASENAME(p))        COMMENT returns input.csv
DISPLAY(EXTENSION(p))       COMMENT returns csv
DISPLAY(DIRNAME(p))         COMMENT returns data/raw (data\raw on Windows)
```

`CWD()` — The current working directory.

`CHDIR(path)` — Changes it. Relative paths used afterwards -- including in
`READFILE` and `WRITEFILE` -- resolve against the new directory, so this changes
the meaning of every relative path in the rest of the program.

`TEMPDIR()` — The system temporary directory.

`HOMEDIR()`, `CONFIGDIR()`, `CACHEDIR()`, `DATADIR()` — The conventional per-user
directories for this platform: `%APPDATA%` and friends on Windows,
`~/Library/Application Support` on macOS, the `XDG_*` locations on Linux. Each
raises an error on a platform that has no such directory for the current user.

## System Integration

### Environment variables

`GETENV(name)` — The value of an environment variable. Errors if it is not set.
Windows matches names case-insensitively, while `ENVVARS()` reports them with the
casing the OS stores.

`GETENV(name, default)` — The value, or `default` if it is not set. The default may
be any value, not just a string.

`SETENV(name, value)` — Sets a variable for this program **and every process it
starts afterwards**. `UNSETENV(name)` removes one; removing a variable that was
never set is not an error.

`ENVVARS()` — Every variable as a dictionary, ordered by name. A variable whose name
or value is not valid text is left out rather than failing the call.

```psl
level <- GETENV("LOG_LEVEL", "info")
SETENV("PYTHONWARNINGS", "ignore")
DISPLAY(LENGTH(ENVVARS()) > 0)
COMMENT GETENV, not CONTAINS(ENVVARS(), ...): Windows stores this key as "Path",
COMMENT and GETENV is the lookup that ignores case.
IF GETENV("PATH", "") NOT= ""
{
    DISPLAY("PATH is set")
}
```

### Running other programs

Both forms wait for the program to finish and return a dictionary with three keys:

| Key | Type | Meaning |
|-----|------|---------|
| `exitcode` | Integer or `NULL` | The exit status; `NULL` if the program was killed by a signal |
| `stdout` | String | Everything it printed to standard output |
| `stderr` | String | Everything it printed to standard error |

`EXEC(program)` or `EXEC(program, argsList)` — Runs a program **directly, with no
shell**. Each element of `argsList` is passed through as one argument exactly as
written, so a filename containing a space, a quote or a `;` arrives intact. This is
the form to reach for by default. On Windows it reaches executables, not `.bat` or
`.cmd` scripts, which need a shell; use `SHELL` for those even though `WHICH` finds
them.

`SHELL(commandLine)` — Runs a command line through the platform's shell: `cmd /C`
on Windows, `sh -c` elsewhere. Use this only when shell syntax -- pipes,
redirection, globbing -- is actually wanted, and remember that the string is
re-parsed by the shell.

`WHICH(program)` — Where a program is on `PATH`, or `NULL` if it is not installed.
Handles the `.exe`/`PATHEXT` lookup on Windows.

```psl
IF WHICH("git") NOT= NULL
{
    r <- EXEC("git", ["rev-parse", "--short", "HEAD"])
    IF r["exitcode"] = 0
    {
        DISPLAY(CONCAT("at commit ", TRIM(r["stdout"])))
    } ELSE
    {
        DISPLAY(CONCAT("git failed: ", r["stderr"]))
    }
}
```

Output is collected in memory rather than streamed, so a command that prints an
enormous amount is best redirected to a file with `SHELL` and then read back with
`READLINES`.

### Processes

`PID()` — This program's own process id.

`PROCESSINFO(pid)` — A dictionary describing one process, or `NULL` if nothing is
running under that id. Keys: `pid`, `name`, `memory` (bytes), `parent` (a pid or
`NULL`).

`PROCESSES()` — Every process the current user can see, as a list of those same
dictionaries, ordered by pid. On a busy machine this is a long list.

`KILL(pid)` — Force-terminates a process: SIGKILL on Unix, `TerminateProcess` on
Windows, with no chance for the target to clean up. Returns `FALSE` if the request
was refused, usually because the process belongs to another user; errors if no
process has that id. It refuses to terminate the interpreter itself -- use `EXIT`.

`EXIT()` or `EXIT(code)` — Ends the program immediately, with exit status `code`
(0--255, defaulting to 0). Buffered output is flushed first, and `TRY` does not catch
it. Run from the `fpli` CLI, `code` becomes the process's exit status; embedded as a
library or in the browser, the program stops and the caller keeps its process and
everything printed.

```psl
me <- PROCESSINFO(PID())
pid <- me["pid"]
used <- me["memory"]
DISPLAY(f"running as pid {pid} using {used} bytes")
```

## Environment Information

Facts about the machine the program is running on. The compile-time ones are always
known; the probed ones are `NULL` on a platform that genuinely cannot report them,
which is worth distinguishing from an empty string.

| Function | Type | Meaning |
|----------|------|---------|
| `PLATFORM()` | String | `"windows"`, `"macos"`, `"linux"`, `"wasi"`, ... |
| `ARCH()` | String | `"x86_64"`, `"aarch64"`, ... |
| `OSFAMILY()` | String | `"windows"`, `"unix"` or `"wasm"` |
| `OSNAME()` | String or `NULL` | The OS's own name for itself |
| `OSVERSION()` | String or `NULL` | Long OS version |
| `KERNELVERSION()` | String or `NULL` | Kernel version |
| `HOSTNAME()` | String or `NULL` | This machine's hostname |
| `USERNAME()` | String or `NULL` | The user running the program |
| `VERSION()` | String | The `fpli` interpreter's version |
| `CPUCOUNT()` | Integer | Logical CPUs available to this program |
| `PHYSICALCPUS()` | Integer or `NULL` | Physical cores |
| `TOTALMEMORY()` | Integer | Total system memory in bytes |
| `USEDMEMORY()` | Integer | Memory in use, in bytes |
| `UPTIME()` | Integer | Seconds since the machine booted |

`SYSINFO()` — All of the above at once, as a dictionary keyed
`platform`, `arch`, `osfamily`, `osname`, `osversion`, `kernelversion`, `hostname`,
`username`, `cpucount`, `physicalcpus`, `totalmemory`, `usedmemory`, `uptime`,
`version`.

```psl
IF PLATFORM() = "windows"
{
    r <- SHELL("dir")
} ELSE
{
    r <- SHELL("ls")
}
DISPLAY(f"{PLATFORM()}/{ARCH()} with {TOSTRING(CPUCOUNT())} CPUs")
```

## Meta Programming

### Inspecting values

`TYPEOF(value)` — The value's type as a string: one of `"integer"`, `"float"`,
`"string"`, `"boolean"`, `"list"`, `"dictionary"`, `"null"`, `"nan"`, or `"unit"`
for the empty value that a procedure without `RETURN` yields.

```psl
DISPLAY(TYPEOF(1))          COMMENT returns integer
DISPLAY(TYPEOF(1.5))        COMMENT returns float
DISPLAY(TYPEOF([1]))        COMMENT returns list
DISPLAY(TYPEOF(NULL))       COMMENT returns null
```

### Running generated code

`EVAL(expression)` — Evaluates a string as a single **expression** and returns its
value.

`EXECUTE(source)` — Runs a string as a whole **program**: statements, assignments
and procedure declarations, all landing in the calling scope. Returns no value.

```psl
EXECUTE("total <- 0")
FOR EACH n IN [1, 2, 3]
{
    EXECUTE("total <- total + " + TOSTRING(n))
}
DISPLAY(total)              COMMENT returns 6
```

Source produced at run time may itself call `EVAL` or `EXECUTE`, but the nesting is
capped: 32 levels, after which the program stops with a clear error rather than
exhausting the interpreter's stack. Each level carries its own lexer, parser and
syntax tree, which is why the limit is far lower than the 1000-deep limit on
ordinary procedure recursion.

### Reaching variables and procedures by name

`ISDEFINED(name)` — Whether a variable of that name is in scope.

`GETVAR(name)` or `GETVAR(name, default)` — The value of a variable named by a
string. Errors without a default if the variable does not exist.

`SETVAR(name, value)` — Creates or updates a variable whose name is computed, in the
current scope, and returns the value assigned. The name must be one that could have
been written in source: a letter followed by letters, digits and underscores, and not
a keyword.

`UNSETVAR(name)` — Removes a variable from the current scope, returning whether there
was one to remove. Like `SETVAR`, it acts on the current scope only, so it cannot
delete a caller's variable.

`VARIABLES()` — The names of every variable in scope, sorted.

`PROCEDURES()` — The names of every declared procedure, sorted.

`CALL(name)` or `CALL(name, argsList)` — Calls a procedure chosen at run time.

```psl
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
```

`CALL` dispatches to procedures declared with `PROCEDURE`, not to built-in
functions -- a built-in's name is known when the program is written, so it never
needs to be looked up. Recursion through `CALL` is bounded by the same limit as any
other recursion.

## Libraries and Multiple Files

`IMPORT libname` or `IMPORT "path/to/libname.psl"`

Runs another `.psl` file and makes everything it declares -- procedures and
variables alike -- available to the importing file. The namespace is flat: there is
no prefix, so a library's names are simply added to yours.

The bare form is the natural spelling for a neighbouring file. A quoted path is
needed for anything containing a directory separator.

**How a file is found.** A relative path is looked for next to the *importing file*
first, and only then relative to the directory `fpli` was run from. A `.psl`
extension is added when the name has none, so `IMPORT "strings"` and
`IMPORT "strings.psl"` mean the same file. An absolute path is used as given. When
nothing matches, the error lists every location tried.

Because resolution follows the importing file, a library can import its own
neighbours and the whole program works no matter which directory it is launched
from.

**Each file runs once.** However many times a file is imported, and by however many
different spellings of its path, its top-level code runs exactly once. That also
makes circular imports terminate: if `a.psl` imports `b.psl` and `b.psl` imports
`a.psl`, the second import is simply skipped. Procedures are looked up when called
rather than when declared, so two files may still use each other's procedures.

A file whose body fails is not recorded, so a later `IMPORT` of it runs it again from
the top -- including any top-level work it had already done before failing. `RETURN`
at a file's top level ends that file and nothing more.

**Where the names land.** An imported file's declarations go into the outermost scope,
whatever scope the `IMPORT` was written in. An `IMPORT` inside a procedure body or a
`CATCH` block therefore still makes its names available to the whole program.

An error raised inside an imported file names that file and shows the offending line
from it.

### Knowing which file you are in

`SCRIPTPATH()` — The absolute path of the file whose code is running. Inside a
procedure this is the file the procedure was **written** in, not the file that
called it, which is what lets a library find its own resources. It is `NULL` when
the program has no location at all -- the library API, or the browser playground.

`ISMAIN()` — `TRUE` only when the running code was written in the file `fpli` was
pointed at. This is PseudoLang's `if __name__ == "__main__"`: a library can carry a
demo or a self-test that stays quiet when the file is imported.

`MODULES()` — The paths of the files imported so far, in import order. The entry
script is not among them.

```psl
COMMENT lib/table.psl
PROCEDURE rows()
{
    COMMENT reads a file sitting next to this library, wherever it was imported from
    RETURN READLINES(JOINPATH(DIRNAME(SCRIPTPATH()), "rows.txt"))
}

IF ISMAIN()
{
    DISPLAY("table.psl self-test")
    DISPLAY(LENGTH(rows()))
}
```

```psl
COMMENT main.psl
IMPORT "lib/table.psl"
DISPLAY(LENGTH(rows()))
DISPLAY(ISMAIN())           COMMENT returns true
DISPLAY(MODULES())          COMMENT returns the absolute path of lib/table.psl
```

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

Procedure recursion is capped at 1000 nested calls, and `EVAL`/`EXECUTE` nesting at
32 levels. Both are reported as errors rather than crashing the interpreter.

Built-in functions are resolved before user-defined procedures, so a procedure
declared with a built-in's name is never called. A *variable* may still be named like
a built-in, since only calls resolve to built-ins.

`CLASS` is parsed but not implemented, and networking is not available yet -- `IMPORT`
reads local files only.

Everything that needs a host process -- file IO, paths, running programs, process
management and the machine facts -- works in the native `fpli` binary and under WASI.
In the browser-embedded WebAssembly build each of those raises an error saying the
sandbox has no filesystem or host process; the rest of the language is unaffected.
