#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
LEXER="$ROOT/src/lexer.rs"
INTERPRETER="$ROOT/src/interpreter.rs"
OUTPUT="$ROOT/utils/tm/pseudolang.tmLanguage.json"

mkdir -p "$(dirname "$OUTPUT")"

# --- Category map ---
declare -A CATEGORY
# Control
for kw in IF ELSE REPEAT UNTIL TIMES FOR EACH IN RETURN PROCEDURE CLASS IMPORT TRY CATCH; do
  CATEGORY[$kw]=control
done
# Constants
for kw in TRUE FALSE NULL NAN; do
  CATEGORY[$kw]=constant
done
# Operators (keyword-form only; symbol operators are static in the template)
for kw in MOD AND OR NOT; do
  CATEGORY[$kw]=operator
done
# Math
for kw in RANDOM ABS CEIL FLOOR POW SQRT SIN COS TAN ASIN ACOS ATAN EXP LOG NLOG LOGTEN LOGTWO GCD FACTORIAL DEGREES RADIANS MIN MAX HYPOT ROUND; do
  CATEGORY[$kw]=math
done
# List
for kw in LENGTH SORT APPEND REMOVE INSERT SPLIT RANGE; do
  CATEGORY[$kw]=list
done
# String
for kw in SUBSTRING CONCAT TRIM REPLACE UPPERCASE LOWERCASE CONTAINS FIND STARTSWITH ENDSWITH; do
  CATEGORY[$kw]=string
done
# IO / misc
for kw in DISPLAY DISPLAYINLINE INPUT TOSTRING TONUM EXIT SLEEP TIME TIMESTAMP TIMEZONE TIMEZONES MILLITIME EVAL HASARG GETARG; do
  CATEGORY[$kw]=io
done

# --- Extract keywords from source ---
extract_keywords() {
  grep -oP '"\K[A-Z][A-Z_0-9]+(?="\s*(=>|\|))' "$1" | sort -u
}

lexer_kws=$(extract_keywords "$LEXER")
interp_kws=$(extract_keywords "$INTERPRETER")
all_kws=$(printf '%s\n%s\n' "$lexer_kws" "$interp_kws" | sort -u)

# --- Bucket keywords by category ---
declare -A BUCKETS
for cat in control constant operator math list string io other; do
  BUCKETS[$cat]=""
done

uncategorized=0
while IFS= read -r kw; do
  [[ -z "$kw" ]] && continue
  # Skip structural tokens that aren't highlighted as keywords
  [[ "$kw" == "COMMENT" || "$kw" == "COMMENTBLOCK" || "$kw" == "NOT=" ]] && continue
  cat="${CATEGORY[$kw]:-other}"
  if [[ "$cat" == "other" ]]; then
    echo "WARNING: uncategorized keyword '$kw' — add to CATEGORY map in $0" >&2
    uncategorized=$((uncategorized + 1))
  fi
  if [[ -n "${BUCKETS[$cat]}" ]]; then
    BUCKETS[$cat]="${BUCKETS[$cat]}|$kw"
  else
    BUCKETS[$cat]="$kw"
  fi
done <<< "$all_kws"

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
if [[ -n "${BUCKETS[control]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Control keywords",
            "match": "\\\\b(${BUCKETS[control]})\\\\b",
            "name": "keyword.control.pseudolang"
        },
EOF
fi

# Constants
if [[ -n "${BUCKETS[constant]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Constants",
            "match": "\\\\b(${BUCKETS[constant]})\\\\b",
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
if [[ -n "${BUCKETS[operator]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Operators",
            "match": "(<-|\\\\+|-|\\\\*|/|=|NOT=|>=|<=|>|<|${BUCKETS[operator]})",
            "name": "keyword.operator.pseudolang"
        },
EOF
fi

# Math functions
if [[ -n "${BUCKETS[math]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Math functions",
            "match": "\\\\b(${BUCKETS[math]})\\\\b",
            "name": "support.function.math.pseudolang"
        },
EOF
fi

# List functions
if [[ -n "${BUCKETS[list]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "List functions",
            "match": "\\\\b(${BUCKETS[list]})\\\\b",
            "name": "support.function.list.pseudolang"
        },
EOF
fi

# String functions
if [[ -n "${BUCKETS[string]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "String functions",
            "match": "\\\\b(${BUCKETS[string]})\\\\b",
            "name": "support.function.string.pseudolang"
        },
EOF
fi

# IO functions
if [[ -n "${BUCKETS[io]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "IO and utility functions",
            "match": "\\\\b(${BUCKETS[io]})\\\\b",
            "name": "support.function.io.pseudolang"
        },
EOF
fi

# Uncategorized (if any)
if [[ -n "${BUCKETS[other]}" ]]; then
  cat >> "$OUTPUT" << EOF
        {
            "comment": "Other builtins (uncategorized)",
            "match": "\\\\b(${BUCKETS[other]})\\\\b",
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
