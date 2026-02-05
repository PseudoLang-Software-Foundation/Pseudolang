ROOT := `git rev-parse --show-toplevel`
VERSION := `grep -m1 '^version' "$(git rev-parse --show-toplevel)/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/'`
