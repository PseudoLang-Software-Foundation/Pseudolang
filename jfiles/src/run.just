import "../common.just"

# Run commands

# Run fplc in debug mode
run *ARGS:
  cd {{ROOT}} && cargo run --features native -- {{ARGS}}

# Run fplc in release mode
run-release *ARGS:
  cd {{ROOT}} && cargo run --release --features native -- {{ARGS}}
