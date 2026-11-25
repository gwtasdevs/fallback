# Contributing

Thank you for considering contributing to Fallback!

## Lua Scripts
Fallback provides a default set of lua scripts found in `./default_scripts/`. Additions & fixes to these are encouraged, provided they are generally useful for everyone.

## Development Setup

1. Fork and clone the repository.
2. Install rust via [rustup](https://rustup.rs)
3. Run `cargo build` to build, the dll should be output to: `./target/i686-pc-windows-msvc/debug/fallback.dll`.
4. Add this dll to GWLauncher or similar to load it when starting Guild Wars.
5. `cargo test` to run tests.
6. `cargo fmt --check` to run format rules.
7. `cargo clippy` to run code quality checks.

## Pull Requests
[Conventional Commits](https://conventionalcommits.org) on pull request titles (and single-commit PR messages) are enforced.
Please format your PR title as:

`<type>(<scope>): <description>`

### Allowed types:
* `feat`: A new feature (triggers a minor version bump).
* `fix`: A bug fix (triggers a patch version bump).
* `perf`: A performance improvement (triggers a patch version bump).
* `docs`: Documentation-only changes.
* `refactor`: Code changes that neither fix a bug nor add a feature.
* `test`: Adding or updating tests.
* `chore` / `ci` / `build` / `style` / `merge`: Maintenance and tooling changes.

For **breaking changes**, add `!` before the colon (e.g., `feat!: change CLI flag behavior`).

### Examples:
* `feat(compass): Add shadow step paths`
* `fix(scripts): Prevent crash when calling async script functions`
* `docs: Fixed various typos`
* `fix!(lua): Updated Lua language version`

## Reverse Engineering
The aim is to have something that does not break when Guild Wars gets an update. To that end, Fallback has minimal interaction with Guild Wars (so there is less reverse engineering code that can break with updates), and all reverse engineering work is done in a resilient way. If you are adding any reverse engineering work, try as much as possible to make it independent of specifics that might change.
For example:
* Try as much as possible to use the GW error assertion strings to find things, as these are less likely to change.
* Scan codes should avoid simply looking for a unique sequence of bytes, and instead be resilient to changes based on what the assembly code is actually doing. If code does a relative jump, the exact number of bytes should not be in the scan pattern. If code uses EDX to do some calculations, is it possible that it might change to ECX or another register with an update?
* There is a comprehensive set of tools for scanning in `./src/gw/scan.rs`. Use these, and add to them if necessary.
* Hard-coded IDs should be avoided: look agents up by string name rather than model ID, UI message ids should be scanned for just like function pointers, etc.
