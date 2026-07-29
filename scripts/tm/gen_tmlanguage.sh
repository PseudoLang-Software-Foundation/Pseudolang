#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
LEXER="$ROOT/src/lexer.rs"
INTERPRETER="$ROOT/src/interpreter.rs"
OUTPUT="$ROOT/utils/tm/pseudolang.tmLanguage.json"

mkdir -p "$(dirname "$OUTPUT")"

# --- Category map ---
# NOTE: portability. /bin/bash on macOS is 3.2.57, which has no associative
# arrays (`declare -A`). Categories are therefore plain space-delimited strings
# in `CAT_<name>` variables, looked up with indirect expansion. To add keywords,
# just extend the relevant CAT_* list below.
CATEGORIES="control constant operator math list dict string io other"

# Control
CAT_control="IF ELSE REPEAT UNTIL TIMES FOR EACH IN RETURN PROCEDURE CLASS IMPORT TRY CATCH"
# Constants
CAT_constant="TRUE FALSE NULL NAN"
# Operators (keyword-form only; symbol operators are static in the template)
CAT_operator="MOD AND OR NOT"
# Math
CAT_math="RANDOM ABS CEIL FLOOR POW SQRT SIN COS TAN ASIN ACOS ATAN EXP LOG NLOG LOGTEN LOGTWO GCD FACTORIAL DEGREES RADIANS MIN MAX HYPOT ROUND"
# List
CAT_list="LENGTH SORT APPEND REMOVE INSERT SPLIT RANGE"
# Dictionary
CAT_dict="DICTIONARY KEYS VALUES HASKEY GETKEY SETKEY REMOVEKEY"
# String
CAT_string="SUBSTRING CONCAT TRIM REPLACE UPPERCASE LOWERCASE CONTAINS FIND STARTSWITH ENDSWITH"
# IO / misc
CAT_io="DISPLAY DISPLAYINLINE INPUT TOSTRING TONUM EXIT SLEEP TIME TIMESTAMP TIMEZONE TIMEZONES MILLITIME EVAL HASARG GETARG"
# Catch-all bucket; never populated by hand.
CAT_other=""

# Return the category a keyword belongs to, or "other" if it is unknown.
category_of() {
  local kw="$1" cat varname words
  for cat in $CATEGORIES; do
    [ "$cat" = "other" ] && continue
    varname="CAT_$cat"
    words="${!varname}"
    case " $words " in
    *" $kw "*)
      printf '%s' "$cat"
      return 0
      ;;
    esac
  done
  printf 'other'
}

# --- Extract keywords from source ---
# NOTE: portability. BSD grep (macOS) has no -P/PCRE support, so \K and
# lookaheads are unavailable. perl is present on both macOS and Linux and gives
# identical semantics: capture the quoted keyword, assert the following "=>" or
# "|" without consuming it.
extract_keywords() {
  perl -nle 'print $1 while /"([A-Z][A-Z_0-9]+)"(?=\s*(?:=>|\|))/g' "$1" | sort -u
}

lexer_kws=$(extract_keywords "$LEXER")
interp_kws=$(extract_keywords "$INTERPRETER")
all_kws=$(printf '%s\n%s\n' "$lexer_kws" "$interp_kws" | sort -u)

# --- Bucket keywords by category ---
# Same bash 3.2 constraint as above: one BUCKET_<cat> scalar per category,
# holding the pipe-delimited alternation used in the generated regexes.
for cat in $CATEGORIES; do
  eval "BUCKET_${cat}=''"
done

# Read the accumulated alternation for a category.
bucket() {
  local varname="BUCKET_$1"
  printf '%s' "${!varname}"
}

uncategorized=0
while IFS= read -r kw; do
  [[ -z "$kw" ]] && continue
  # Skip structural tokens that aren't highlighted as keywords
  [[ "$kw" == "COMMENT" || "$kw" == "COMMENTBLOCK" || "$kw" == "NOT=" ]] && continue
  cat="$(category_of "$kw")"
  if [[ "$cat" == "other" ]]; then
    echo "WARNING: uncategorized keyword '$kw' — add to the CAT_* lists in $0" >&2
    uncategorized=$((uncategorized + 1))
  fi
  if [[ -n "$(bucket "$cat")" ]]; then
    eval "BUCKET_${cat}=\"\${BUCKET_${cat}}|\${kw}\""
  else
    eval "BUCKET_${cat}=\"\${kw}\""
  fi
done <<<"$all_kws"

# --- Helper: format keyword list as regex ---
# Input: pipe-delimited keywords. Output: escaped for JSON regex.
to_regex() {
  echo "$1"
}

# --- Generate JSON ---
cat > "$OUTPUT" << 'TMHEADER'
{
    "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
    "name": "PseudoLang",
    "scopeName": "source.pseudolang",
    "fileTypes": ["psl"],
    "patterns": [
        {
            "comment": "Line comments (// and # style)",
            "match": "(//|#).*$",
            "name": "comment.line.pseudolang"
        },
        {
            "comment": "Block comments",
            "begin": "COMMENTBLOCK",
            "end": "COMMENTBLOCK",
            "name": "comment.block.pseudolang"
        },
        {
            "comment": "Line comments",
            "match": "COMMENT.*$",
            "name": "comment.line.pseudolang"
        },
TMHEADER

# Control keywords
if [[ -n "$(bucket control)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Control keywords",
            "match": "\\\\b($(bucket control))\\\\b",
            "name": "keyword.control.pseudolang"
        },
EOF
fi

# Constants
if [[ -n "$(bucket constant)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Constants",
            "match": "\\\\b($(bucket constant))\\\\b",
            "name": "constant.language.pseudolang"
        },
EOF
fi

# Numbers (static)
cat >> "$OUTPUT" << 'EOF'
        {
            "comment": "Numbers",
            "match": "\\b\\d+(\\.\\d+)?\\b",
            "name": "constant.numeric.pseudolang"
        },
EOF

# Operators (symbol + keyword)
if [[ -n "$(bucket operator)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Operators",
            "match": "(<-|\\\\+|-|\\\\*|/|=|NOT=|>=|<=|>|<|$(bucket operator))",
            "name": "keyword.operator.pseudolang"
        },
EOF
fi

# Math functions
if [[ -n "$(bucket math)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Math functions",
            "match": "\\\\b($(bucket math))\\\\b",
            "name": "support.function.math.pseudolang"
        },
EOF
fi

# List functions
if [[ -n "$(bucket list)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "List functions",
            "match": "\\\\b($(bucket list))\\\\b",
            "name": "support.function.list.pseudolang"
        },
EOF
fi

# Dictionary functions
if [[ -n "$(bucket dict)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Dictionary functions",
            "match": "\\\\b($(bucket dict))\\\\b",
            "name": "support.function.dict.pseudolang"
        },
EOF
fi

# String functions
if [[ -n "$(bucket string)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "String functions",
            "match": "\\\\b($(bucket string))\\\\b",
            "name": "support.function.string.pseudolang"
        },
EOF
fi

# IO functions
if [[ -n "$(bucket io)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "IO and utility functions",
            "match": "\\\\b($(bucket io))\\\\b",
            "name": "support.function.io.pseudolang"
        },
EOF
fi

# Uncategorized (if any)
if [[ -n "$(bucket other)" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Other builtins (uncategorized)",
            "match": "\\\\b($(bucket other))\\\\b",
            "name": "support.function.other.pseudolang"
        },
EOF
fi

# Static patterns: strings, lists, braces
cat >> "$OUTPUT" << 'EOF'
        {
            "comment": "Multiline string",
            "begin": "\"\"\"",
            "end": "\"\"\"",
            "name": "string.quoted.triple.pseudolang"
        },
        {
            "comment": "Formatted string",
            "begin": "f\"",
            "end": "\"",
            "name": "string.quoted.double.format.pseudolang",
            "patterns": [
                {
                    "match": "\\{[^}]*\\}",
                    "name": "variable.other.pseudolang"
                },
                {
                    "match": "\\\\[ntrb\"\\\\]",
                    "name": "constant.character.escape.pseudolang"
                }
            ]
        },
        {
            "comment": "Raw string",
            "begin": "r\"",
            "end": "\"",
            "name": "string.quoted.double.raw.pseudolang"
        },
        {
            "comment": "Regular string",
            "begin": "\"",
            "end": "\"",
            "name": "string.quoted.double.pseudolang",
            "patterns": [
                {
                    "match": "\\\\[ntrb\"\\\\]",
                    "name": "constant.character.escape.pseudolang"
                }
            ]
        },
        {
            "comment": "Lists",
            "begin": "\\[",
            "end": "\\]",
            "patterns": [{ "include": "$self" }],
            "name": "meta.structure.list.pseudolang"
        },
        {
            "comment": "Block braces",
            "match": "[{}]",
            "name": "punctuation.section.block.pseudolang"
        },
        {
            "comment": "Dictionary key-value separator",
            "match": ":",
            "name": "punctuation.separator.key-value.pseudolang"
        },
        {
            "comment": "Parentheses",
            "match": "[()]",
            "name": "punctuation.section.parens.pseudolang"
        }
    ]
}
EOF

echo "Generated $OUTPUT"
if [[ $uncategorized -gt 0 ]]; then
  echo "WARNING: $uncategorized uncategorized keyword(s) found — update the CATEGORY map" >&2
  exit 1
fi
