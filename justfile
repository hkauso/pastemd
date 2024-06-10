build database="sqlite":
    cargo build -r --no-default-features --features {{database}}

test:
    cargo run
