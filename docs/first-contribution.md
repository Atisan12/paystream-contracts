# Your First Contribution to PayStream

New to open source, Rust or Soroban? This guide takes you from zero to an open pull request. For full project rules see [CONTRIBUTING.md](../CONTRIBUTING.md).

## 1. Fork and clone

Click **Fork** on [the repository](https://github.com/veracindarella/paystream-contracts), then:

```console
$ git clone https://github.com/<your-username>/paystream-contracts.git
$ cd paystream-contracts
$ git remote add upstream https://github.com/veracindarella/paystream-contracts.git
$ git remote -v
origin    https://github.com/<your-username>/paystream-contracts.git (fetch)
origin    https://github.com/<your-username>/paystream-contracts.git (push)
upstream  https://github.com/veracindarella/paystream-contracts.git (fetch)
upstream  https://github.com/veracindarella/paystream-contracts.git (push)
```

## 2. Install prerequisites

**Rust** (the repo's `rust-toolchain.toml` picks the right version):

```console
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ rustup target add wasm32-unknown-unknown
$ rustc --version
rustc 1.xx.x (...)
```

**Stellar CLI:**

```console
$ cargo install --locked stellar-cli
$ stellar --version
stellar 2x.x.x
```

Platform-specific notes (macOS, Linux, Windows, Docker) are in [CONTRIBUTING.md § Development Setup](../CONTRIBUTING.md#development-setup).

## 3. Run the tests

```console
$ cargo test
...
test result: ok. N passed; 0 failed; 0 ignored
```

If everything passes, your setup works.

## 4. Pick a good first issue

1. Open [`good-first-issue` issues](https://github.com/veracindarella/paystream-contracts/issues?q=is%3Aissue+is%3Aopen+label%3Agood-first-issue).
2. Comment that you're taking it and wait to be assigned.
3. Good starting points: fixing docs in `docs/`, adding a test in `contracts/stream/src/test.rs`, or a small self-contained feature.

## 5. Make your change

```console
$ git checkout -b docs/fix-typo-in-faq
# edit files...
$ cargo fmt && cargo clippy && cargo test
$ git add -A
$ git commit -m "docs: fix typo in FAQ"
$ git push origin docs/fix-typo-in-faq
```

Commit messages follow [Conventional Commits](../CONTRIBUTING.md#commit-conventions).

## 6. Open a pull request

On GitHub, click **Compare & pull request**. In the description:

- Say what changed and why.
- Add `Closes #<issue-number>`.
- Confirm tests pass.

A maintainer will review it. Push more commits to the same branch to respond to feedback.

## Getting help

Ask in [GitHub Discussions](https://github.com/veracindarella/paystream-contracts/discussions). See [bounties.md](bounties.md) for how merged work is rewarded.
