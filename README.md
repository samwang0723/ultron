# ultron
Stock information crawler

## Installation
```bash
cargo clean
cargo build
```

## Execution
Depends on your build binary location
```bash
ultron --target=daily_close --date=20240723
ultron --target=three_primary --date=20240723
ultron --target=concentration
```

## Build docker image
```bash
make docker-build
```
