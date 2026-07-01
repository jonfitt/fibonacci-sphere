# Contributing

Thank you for your interest in contributing to **fibonacci_sphere**.

## How to contribute

This project uses a **fork and pull request** workflow. Outside contributors do not receive write access to the repository; all changes go through a reviewed pull request.

1. **Fork** the repository on GitHub.
2. **Create a branch** on your fork (do not open pull requests from your fork's `main` branch).
3. **Make your changes** and add or update tests where appropriate.
4. **Run checks locally** before opening a PR:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
5. **Open a pull request** against `main` on the upstream repository.
6. Wait for **CI to pass** and for a **maintainer review**. A maintainer will merge approved changes.

## Pull request guidelines

- Keep pull requests focused on a single change or feature.
- Use a clear title and description. The PR template prompts for the important details.
- Link related issues when applicable (`Fixes #123`).
- Ensure `cargo fmt`, `cargo clippy`, and `cargo test` pass locally.
- Respond to review feedback and resolve conversation threads.

## Branch protection

The `main` branch is protected:

- All changes must go through a pull request.
- At least one approving review from a code owner is required.
- The `build` CI workflow must pass on the latest commit.
- Only the repository owner can merge pull requests.
- Merge commits are not allowed on `main`; approved PRs are squash-merged.

## Reporting security issues

Do **not** open a public issue for security vulnerabilities. See [SECURITY.md](./SECURITY.md) for responsible disclosure instructions.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project ([GPL-3.0-or-later](./LICENSE.md)).
