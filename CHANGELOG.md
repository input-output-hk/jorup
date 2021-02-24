# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Cannot use `--make-default` more than once (#32).

## [0.6.0] - 2020-06-25

### Added

- Add `jorup setup update` for updating `jorup` itself with a simple command. Updates are checked
  when `jorup` is starting.
- Wallet management:
  - Jormungandr secret keys can be imported with `jorup wallet itn --import secret-file-name`.
  - `jorup wallet` now outputs address, public key and private key location.
  - `jorup wallet` generates BFT secrets file when called.

### Removed

- Support for `jormungandr` node ID.

[Unreleased]: https://github.com/input-output-hk/jorup/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/input-output-hk/jorup/compare/v0.0.5...v0.0.6
