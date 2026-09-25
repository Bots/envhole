# Releasing EnvHole

Releases are built by GitHub Actions from an annotated semantic-version tag on
`main`. The workflow publishes statically linked Linux archives for x86-64 and
ARM64 together with a SHA-256 checksum for each archive. Repository release
immutability must remain enabled so published tags, assets, and automatically
generated release attestations cannot be replaced.

## Prepare

1. Confirm `Cargo.toml` contains the intended version.
2. Merge the version and release notes through a reviewed pull request.
3. From a clean, current `main`, run the same local validation as CI:

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo audit
cargo build --release --locked
```

## Publish

For version `0.1.0`:

```sh
git switch main
git pull --ff-only origin main
git tag -a v0.1.0 -m "EnvHole v0.1.0"
git push origin v0.1.0
```

The release workflow rejects malformed tags and tags that do not match the
package version. It builds and smoke-tests each native binary before the
publish job starts. The publish job rechecks every archive checksum and then
creates the GitHub release with generated release notes.

Verify the result rather than trusting the workflow exit status:

```sh
gh release view v0.1.0 --repo Bots/envhole
gh release verify v0.1.0 --repo Bots/envhole
gh release download v0.1.0 --repo Bots/envhole --dir /tmp/envhole-release
(
  cd /tmp/envhole-release
  sha256sum --check --strict ./*.sha256
)
```

Do not move or reuse a published release tag. If a release needs correction,
bump the patch version and publish a new tag.
