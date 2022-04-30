<h1 align="center">
  Bulbasaur
</h1>

A toy payments engine.

## Getting started

You can clone the project in your system and run with this command.

```sh
cargo run -- testdata/transactions.csv

# Output
client,available,held,total,locked
1,1.5,0.0,1.5,false
2,2.0,0.0,2.0,false
```

## Considerations

### Basic

We can build the project with `cargo build --release` command.

It can accept CSV files with any type of spacing and returns CSV output to stdout.

### Completeness

It has support for the following transactions:

- **deposit** - to deposit money.
- **withdrawal** - to withdraw money.
- **dispute** - raise dispute before resolving or chargeback.
- **resolve** - when a dispute is resolved.
- **chargeback** - when a dispute is charged back, account gets locked.

### Correctness

There are many test cases to ensure that the logic is correctly implemented. You can find them in `app.rs` file.

**Precision** - To handle precision, we only consider the result it till the four digit _round off_ after the decimal point. Ex:

```rust
3.66666 + 3.66666;

// Result 7.3334
```

### Safety and Robustness

There isn't anything we are doing which dangerous.

### Efficiency

The [csv](https://docs.rs/csv/latest/csv/) library creates the buffer around the file. Since the entire file isn't loaded in the memory we can send the file of bigger sizes too.

For TCP connections too we can create a buffer around them and pass it to the reader. Right now it doesn't handles the requests concurrently but sure we can implement in future.

### Maintainability

Since it's a small project it's also well maintained:

- **`app.rs`** - contains main business logic.
- **`cli_config.rs`** - configuration related to CLI.
- **`main.rs`** - main entry point for the application.
