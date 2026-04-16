# Contributing to Orph

This is a Core Red project. All contributions are subject to review, approval, and compliance with the project's standards.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Code Style](#code-style)
- [Commit Guidelines](#commit-guidelines)

## Code of Conduct

This project adheres to a Code of Conduct. By participating, you are expected to uphold this code. Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## Getting Started

1. Fork the repository
2. Clone your fork locally
3. Set up the development environment
4. Create a branch for your changes
5. Make your changes
6. Test your changes
7. Submit a pull request

## Development Setup

### Prerequisites

- **Rust** (1.85+) — [Install](https://rustup.rs/)

### Building

```bash
# Clone the repository
git clone https://github.com/CoreRed-Project/orph-cli.git
cd orph-cli

# Build both binaries
make build

# Run clippy and fmt checks
make lint
make fmt

# Build in release mode
make release
```

## How to Contribute

### Reporting Bugs

Before creating a bug report, check existing issues to avoid duplicates.

When reporting a bug, include:

- **Clear title**: Descriptive summary of the issue
- **Steps to reproduce**: Detailed steps to reproduce the behavior
- **Expected behavior**: What you expected to happen
- **Actual behavior**: What actually happened
- **Environment**: OS, Rust version, `orph --version` output
- **Screenshots**: If applicable

### Suggesting Features

Feature suggestions are accepted. Provide:

- **Clear title**: Descriptive summary of the feature
- **Use case**: Why this feature would be useful
- **Proposed solution**: How you envision the feature working
- **Alternatives considered**: Other solutions you have evaluated

### Code Contributions

We accept contributions for:

- Bug fixes
- Performance improvements
- Documentation improvements
- New features (discuss first via an issue)

## Pull Request Process

1. **Create an issue first** for significant changes
2. **Fork and branch**: Create a feature branch from `main`
3. **Follow code style**: Ensure your code follows our style guidelines
4. **Write tests**: Add tests for new functionality
5. **Update documentation**: Update relevant documentation
6. **Run all checks**: Ensure `make lint` and `make fmt` pass
7. **Commit properly**: Follow commit message guidelines
8. **Submit PR**: Create a pull request with a clear description

### PR Requirements

- [ ] Code compiles without warnings (`cargo build`)
- [ ] No clippy warnings (`make lint`)
- [ ] Code is formatted (`make fmt`)
- [ ] Documentation is updated where relevant
- [ ] Commit messages follow guidelines
- [ ] PR description explains the changes

## Code Style

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting: `make fmt`
- Use `clippy` for linting: `make lint`
- Write documentation comments for public APIs
- Keep functions focused and small
- Use `anyhow::Result` for error handling — no `unwrap()` in production paths

## Commit Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/).

### Format

```text
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Examples

```text
feat(daemon): add pet.rename IPC handler

Implements the pet.rename command in the daemon IPC layer,
delegating to pet_service::rename.

Closes #42
```

```text
fix(run): reject script names with leading dots

Path traversal safety: script names starting with '.' are now
rejected with a clear error message.
```

```text
docs: add telemetry opt-out instructions to README
```

### Rules

- Use imperative mood ("add" not "added")
- Do not capitalize the first letter
- No period at the end of the subject
- Limit subject line to 50 characters
- Wrap body at 72 characters

## Questions

If you have questions:

- Open an issue with the `question` label
- Start a discussion in the repository
