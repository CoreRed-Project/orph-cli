# Security Policy

## Supported Versions

Orph is currently in active development. Security updates will be provided for the following versions:

| Version | Supported |
|---|---|
| v0.1.0 | Yes |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in Orph, report it responsibly.

### How to Report

**DO NOT** open a public issue for security vulnerabilities.

Instead, email security concerns to:

**security.sxnnyside@sxnnysideproject.com**

Or open a private security advisory on GitHub:
https://github.com/CoreRed-Project/orph-cli/security/advisories/new

### What to Include

When reporting a vulnerability, include:

1. **Description**: Clear description of the vulnerability
2. **Impact**: Potential impact and affected versions
3. **Reproduction**: Step-by-step instructions to reproduce
4. **Proof of Concept**: Code or example demonstrating the issue
5. **Suggested Fix**: If you have ideas on how to fix it

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depending on severity, typically within 30 days

### Disclosure Policy

- Allow the maintainers time to fix the vulnerability before public disclosure
- We will credit you in the security advisory (unless you prefer to remain anonymous)
- We may request your help in validating the fix

## Security Considerations

### Local Data Storage

Orph stores all data locally in `~/.orph/orph.db` and `~/.orph/orph.log`. No data is transmitted externally.

- SQLite database contains pet state, config keys, and local telemetry
- Log file contains structured command history and error records
- No credentials, tokens, or sensitive values are stored by default

### Script Execution

`orph run` executes scripts from `~/.orph/scripts/` only.

- Path traversal is rejected (no `/`, `\`, or leading `.` in script names)
- Scripts are not executed via the daemon — execution is CLI-side only
- Users are responsible for the content of their own scripts

### Dependencies

We regularly update dependencies to address known vulnerabilities. Dependency audit is performed via `cargo audit` before each release.

## Known Security Limitations

### 1. Unix Socket Permissions

`orphd` binds to `/tmp/orphd.sock`. Any local user with access to `/tmp/` can send IPC commands to the daemon.

Mitigation: Do not run `orphd` on multi-user systems where other users are untrusted.

## Best Practices

### For Users

1. **Review Scripts**: Always review scripts in `~/.orph/scripts/` before running them
2. **Trusted Sources**: Only place scripts from trusted sources in the scripts directory
3. **Update Regularly**: Keep Orph updated to the latest version
4. **Socket Awareness**: Be aware of `/tmp/orphd.sock` permissions on shared systems

### For Developers

1. **Input Validation**: Always validate input before passing to orph
2. **Error Handling**: Handle all errors returned by the CLI or IPC layer
3. **Update Dependencies**: Regularly audit and update dependencies

## Threat Model

### In Scope

- Path traversal vulnerabilities in script execution
- IPC input handling (malformed JSON, unknown commands)
- Memory safety issues
- Dependency vulnerabilities

### Out of Scope

- Execution of user-provided scripts (user responsibility)
- Denial of service via large inputs
- Social engineering attacks
- Physical access to the user's machine

## Security Updates

Security updates will be announced via:

1. GitHub Security Advisories
2. Release notes in CHANGELOG.md
3. GitHub Releases page

Critical security updates will be backported to supported versions when feasible.