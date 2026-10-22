# saleae_cli: the Saleae logic analyzer library, CLI and Python module.

py_venv := "/tmp/saleae_py-venv"

default:
    @just --list

# The tests. `just test`: all of them (test-all), the run of the `tpm land` gate. `just test --changed`: only the
# workspace crates changed against main and the crates that depend on them (tpm test changed), what a session runs
# before tpm work ready.
test *flags:
    if [ "{{flags}}" = "--changed" ]; then tpm test changed -- cargo nextest run --workspace; else {{just_executable()}} test-all {{flags}}; fi

# Tests, Rust and Python (tests/sim.rs and the Python smoke test skip without `saleae server install`)
test-all: test-py
    cargo test --workspace --quiet

# Build the Python module (crates/saleae_py) into a venv under /tmp and run its pytest smoke test
[working-directory('crates/saleae_py')]
test-py:
    UV_PROJECT_ENVIRONMENT={{ py_venv }} uv run --quiet --group dev pytest -q

# Format check and clippy without warnings, all crates
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# With tpm on PATH: tpm install (kept-releases stack, one copy on PATH); otherwise: cargo install
# Build and install saleae on this PC; completion: source <(COMPLETE=bash saleae)
install:
    cargo build --profile deploy --locked -p saleae_cli
    if command -v tpm >/dev/null; then tpm install target/deploy/saleae; else cargo install --locked --path crates/saleae_cli; fi

# Bring this PC up to main: pull, install, check --version against main
deploy:
    if command -v tpm >/dev/null; then tpm repos pull saleae_rs; else git pull --ff-only; fi
    just install
    saleae --version | grep -q "($(git rev-parse --short HEAD)" || { echo "saleae --version does not show $(git rev-parse --short HEAD)"; saleae --version; exit 1; }
    saleae --version
