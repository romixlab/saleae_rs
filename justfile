# saleae_cli: the Saleae logic analyzer library, CLI and Python module.

py_venv := "/tmp/saleae_py-venv"

default:
    @just --list

# Tests, Rust and Python (tests/sim.rs and the Python smoke test skip without `saleae server install`)
test: test-py
    cargo test --workspace --quiet

# Build the Python module (crates/saleae_py) into a venv under /tmp and run its pytest smoke test
[working-directory('crates/saleae_py')]
test-py:
    UV_PROJECT_ENVIRONMENT={{ py_venv }} uv run --quiet --group dev pytest -q

# Format check and clippy without warnings, all crates
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# Our PCs: the kept-releases stack, one copy on PATH; anywhere else: plain cargo install
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
