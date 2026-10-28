//! Fetches Saleae's automation API proto (Apache-2.0, from the logic2-automation repo on GitHub, pinned to a commit
//! and a SHA-256) into `OUT_DIR` and compiles it with protox (pure Rust, no `protoc`) into a tonic client.
//!
//! - `SALEAE_PROTO_DIR=DIR`: compile `DIR/saleae/grpc/saleae.proto` instead, without download or checksum
//!   (offline builds; or the API 1.2 proto from the server zip, `automation_server/proto`, for Logic MSO work).
//! - `DOCS_RS`: docs.rs builds have no network, so the proto is skipped and the crate is built with
//!   `cfg(saleae_stub_proto)`: no modules, since none of them can exist without the generated types.
//!
//! The download uses `ureq` (same as `saleae_rs::server::install`), https only; integrity comes from the pinned
//! SHA-256, not from TLS.

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs};

/// saleae/logic2-automation tag v1.0.11 (API 1.0.0), the latest published proto.
const PROTO_COMMIT: &str = "6b3bd221a688b1dc580b4a1bfc0e74d8dcb1380d";
const PROTO_SHA256: &str = "13ed873e9d4eb6eac222ef4e532a39f4c330da9ea45c4e998a1805650a15311e";
const PROTO_REL: &str = "saleae/grpc/saleae.proto";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SALEAE_PROTO_DIR");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rustc-check-cfg=cfg(saleae_stub_proto)");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let include = match env::var_os("SALEAE_PROTO_DIR") {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            println!("cargo:rerun-if-changed={}", dir.join(PROTO_REL).display());
            dir
        }
        None if env::var_os("DOCS_RS").is_some() => {
            println!("cargo:warning=DOCS_RS set: no network, building without the Saleae proto");
            println!("cargo:rustc-cfg=saleae_stub_proto");
            return;
        }
        None => {
            let dir = out.join("proto");
            fetch_proto(&dir.join(PROTO_REL));
            dir
        }
    };
    let proto = include.join(PROTO_REL);
    let fds = protox::compile([&proto], [&include])
        .unwrap_or_else(|e| panic!("compile {}: {e}", proto.display()));
    tonic_prost_build::configure()
        .build_server(false)
        .compile_fds(fds)
        .expect("generate gRPC client");
}

/// Downloads the pinned proto to `dest` unless it is already there with the right checksum.
fn fetch_proto(dest: &Path) {
    if sha256_file(dest).as_deref() == Some(PROTO_SHA256) {
        return;
    }
    let url = format!(
        "https://raw.githubusercontent.com/saleae/logic2-automation/{PROTO_COMMIT}/proto/{PROTO_REL}"
    );
    assert!(url.starts_with("https://"), "proto url {url} is not https");
    let parent = dest.parent().expect("proto path has a parent");
    fs::create_dir_all(parent).expect("create proto dir in OUT_DIR");
    let tmp = dest.with_extension("proto.part");
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .new_agent();
    let body = agent
        .get(&url)
        .call()
        .unwrap_or_else(|e| {
            panic!(
                "download of {url} failed ({e}); offline: set SALEAE_PROTO_DIR to a directory holding \
                 {PROTO_REL}"
            )
        })
        .into_body()
        .read_to_vec()
        .unwrap_or_else(|e| panic!("read {url}: {e}"));
    fs::write(&tmp, &body).expect("write downloaded proto");
    let got = sha256_file(&tmp).unwrap_or_default();
    if got != PROTO_SHA256 {
        let _ = fs::remove_file(&tmp);
        panic!("{url}: SHA-256 {got} does not match the pinned {PROTO_SHA256}; refusing to build");
    }
    fs::rename(&tmp, dest).expect("move proto into place");
}

fn sha256_file(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let data = fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(data)))
}
