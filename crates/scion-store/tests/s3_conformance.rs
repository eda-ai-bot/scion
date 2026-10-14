//! Conformance suite against a real S3-compatible object store.
//!
//! Requires `--features integration` and a reachable server with the bucket
//! already created (default: the `RustFS` instance from
//! `deploy/docker-compose.yml`). Override with `SCION_TEST_S3_ENDPOINT`,
//! `SCION_TEST_S3_BUCKET`, `SCION_TEST_S3_ACCESS_KEY`, `SCION_TEST_S3_SECRET_KEY`.
//! Each test writes under its own prefix.

#![cfg(feature = "integration")]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use object_store::aws::AmazonS3Builder;
use object_store::prefix::PrefixStore;
use scion_store::ObjectStorage;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn fresh() -> ObjectStorage {
    let s3 = AmazonS3Builder::new()
        .with_endpoint(env_or("SCION_TEST_S3_ENDPOINT", "http://localhost:9000"))
        .with_bucket_name(env_or("SCION_TEST_S3_BUCKET", "scion-objects"))
        .with_access_key_id(env_or("SCION_TEST_S3_ACCESS_KEY", "scion"))
        .with_secret_access_key(env_or("SCION_TEST_S3_SECRET_KEY", "scionscion"))
        .with_region("us-east-1")
        .with_allow_http(true)
        .with_virtual_hosted_style_request(false)
        .build()
        .expect("valid S3 configuration");
    let prefix = format!(
        "test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    ObjectStorage::new(Arc::new(PrefixStore::new(s3, prefix)))
}

scion_store::object_conformance_tests!(s3, async { fresh() });
