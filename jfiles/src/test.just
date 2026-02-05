import "../common.just"

# Test commands

# Run all tests
test:
  cd {{ROOT}} && cargo test

# Run tests with output
test-verbose:
  cd {{ROOT}} && cargo test -- --nocapture
