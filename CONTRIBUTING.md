# Contributing to Only1MCP

Thank you for your interest in contributing to Only1MCP! We welcome contributions from the community and are grateful for any help you can provide.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:
- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive criticism
- Accept feedback gracefully

## How to Contribute

### Reporting Issues

1. **Check existing issues** first to avoid duplicates
2. **Use issue templates** when available
3. **Provide details**:
   - Only1MCP version (`only1mcp --version`)
   - Operating system and version
   - Steps to reproduce
   - Expected vs actual behavior
   - Error messages and logs

### Suggesting Features

1. **Open a discussion** first for major features
2. **Explain the use case** and why it would benefit users
3. **Consider implementation complexity** and maintenance burden

### Submitting Code

#### Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/doublegate/Only1MCP.git
cd only1mcp

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build

# Run tests
cargo test

# Run lints
cargo clippy

# Format code
cargo fmt
```

#### Development Workflow

1. **Fork the repository** on GitHub
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**:
   - Write clear, self-documenting code
   - Add tests for new functionality
   - Update documentation as needed
4. **Test your changes**:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```
5. **Commit with meaningful messages**:
   ```bash
   git commit -m "feat: add new load balancing algorithm"
   ```
6. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```
7. **Open a Pull Request** with:
   - Clear description of changes
   - Link to related issues
   - Test results/screenshots if applicable

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `style:` Code style changes (formatting, etc)
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `test:` Test additions or corrections
- `chore:` Maintenance tasks
- `ci:` CI/CD changes

Examples:
```
feat: add WebSocket transport support
fix: resolve memory leak in connection pool
docs: update configuration examples
perf: optimize request batching algorithm
```

## Development Guidelines

### Code Style

- Follow Rust standard style guidelines
- Use `cargo fmt` to format code
- Run `cargo clippy` and address all warnings
- Write descriptive variable and function names
- Add comments for complex logic
- Document public APIs with doc comments

### Testing

- Write unit tests for new functions
- Add integration tests for new features
- Ensure all tests pass: `cargo test`
- Aim for high test coverage
- Test edge cases and error conditions

### Running Benchmarks

Before submitting performance-related PRs, run benchmarks to validate changes:

```bash
# Run all benchmarks
cargo bench

# Save baseline before changes
cargo bench -- --save-baseline before-changes

# Make your changes...

# Compare against baseline
cargo bench -- --baseline before-changes
```

**Performance Regression Guidelines**:
- <5% regression: Generally acceptable (document reason in PR)
- 5-10% regression: Needs justification and approval
- >10% regression: **Not acceptable** without strong justification

See [Performance Benchmarking Guide](docs/performance_benchmarking.md) for comprehensive documentation.

### Performance

- Profile before optimizing
- Document performance-critical sections
- Use appropriate data structures
- Minimize allocations in hot paths
- Benchmark significant changes (use `cargo bench`)

### Security

- Never commit secrets or credentials
- Validate all inputs
- Use secure defaults
- Follow OWASP guidelines
- Report security issues privately

## Project Structure

See [CLAUDE.md](CLAUDE.md) for detailed project structure and architecture information.

## Pull Request Process

1. **Ensure CI passes** - All checks must be green
2. **Update documentation** - README, CHANGELOG, etc.
3. **Add tests** - New features need test coverage
4. **Request review** - From maintainers or contributors
5. **Address feedback** - Make requested changes
6. **Squash commits** - If requested by maintainers
7. **Wait for approval** - Two approvals required for merge

### PR Review Criteria

- **Code quality**: Clean, readable, maintainable
- **Tests**: Adequate coverage, passing
- **Documentation**: Updated and accurate
- **Performance**: No regressions
- **Security**: No vulnerabilities introduced
- **Design**: Follows project architecture

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release tag: `v0.x.y`
4. Build release binaries
5. Publish to crates.io
6. Create GitHub release

## Getting Help

- **Documentation**: Read the [docs/](docs/) folder
- **Discussions**: Open a GitHub Discussion
- **Discord**: Join our community server
- **Issues**: Check existing issues or open new ones

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- GitHub contributors page
- Release notes
- Project documentation

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

## Questions?

Feel free to open an issue or discussion if you have any questions about contributing!

Thank you for helping make Only1MCP better! 🚀