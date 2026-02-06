import "../common.just"

# Run commands

# Run fpli in debug mode
run *ARGS:
  cd {{ROOT}} && cargo run --features native -- {{ARGS}}

# Run fpli in release mode
run-release *ARGS:
  cd {{ROOT}} && cargo run --release --features native -- {{ARGS}}
