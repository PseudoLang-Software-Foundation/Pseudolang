# PseudoLang Guide

## Assignment, Display, Input

`a <- b`

Evaluates b and then assigns a copy of the result to the variable a

`DISPLAY(a)`

Prints the value of a.

`INPUT()`

Accepts a value from the user (command line) and returns the input value.

## Arithmetic Operators and Numeric Procedures

`a + b`

`a - b`

`a * b`

`a / b`

`a MOD b`

The arithmetic operators +, -, *, /, and MOD are used to perform arithmetic on a and b.

`RANDOM(a, b)`

Generates and returns a random integer from a to b, including a and b. Each result is equally likely to occur.

## Relational and Boolean Operators

`a = b`

`a NOT= b`

`a > b`

`a < b`

`a >= b`

`a <= b`

The relational operators =, !=, >, <, >=, and <= are used to test the relationship between two variables, expressions, or values. A comparison using relational operators evaluates to a Boolean value.

`a AND b`

Evaluates to true if both a and b are true; otherwise evaluates to false.

`a OR b`

Evaluates to true if a is true or if b is true or if both a and b are true; otherwise evaluates to false.

`NOT a`

Evaluates to true if a is false; otherwise evaluates to false.

## Selection

```text
IF(a)
{
 <statement(s)>
}
```

The code in block of statements is executed if the Boolean expression a evaluates to true; no action is taken if condition evaluates to false.

```text
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

```text
REPEAT n TIMES
{
 <statement(s)>
}
```

The code inside is executed n times.

```text
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

```text
FOR EACH item IN aList
{
 <statement(s)>
}
```

The variable item is assigned the value of each element of aList sequentially, in order, from the first element to the last element. The statements are executed once for each assignment of item.

```text
PROCEDURE procName(a, b)
{
 <statement(s)>
}
```

Defines procName as a procedure that takes zero or more arguments. The procedure contains statements. The procedure procName can be called using the following notation, where arg1 is assigned to parameter1, arg2 is assigned to parameter2 and so on.
`procName(arg1, arg2)`

`RETURN(a)`

Returns the flow of control to the point where the procedure was
called and returns the value of a. Can be used as a value itself.

## Data Types

`1`

Integer (32 bit)

`0.1`

Float (32 bit)

`"a"`

String (32 bit)

`TRUE` or `FALSE`

Boolean

## BuiltIn Functions
`Substring("abcd", start, end)`
Returns a string of characters from index `start` to index `end` of the given string

`Concat("ab", "cd")`
Returns a single string with the two given strings combined


## Outside of College Board - Extras

```text
COMMENT a
```

```text
COMMENTBLOCK
a
b
COMMENTBLOCK
```

A comment (multi-line or single-line), anything on the line after this or in between does not affect the code.

`DISPLAYINLINE(a)`

Prints the value of a, without a new line character.

`IMPORT a`

Imports a library (including functions & variables defined in that file) from a file.

```
CLASS className()
{
  ...procs
}
```
Creates a class object

## Limitations

Since a lot of the syntax is text like COMMENT or TRUE, you may not set variables as such, and the interpreter will try to raise an error if it occurs.
