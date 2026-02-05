# Main justfile - imports the jfiles justfile

import "jfiles/justfile"

@default:
    @echo "Justfiles"
    @echo "================"
    @just --list