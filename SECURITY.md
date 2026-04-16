# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2.0.x (Pecu 2.0 / 3.0 Themis) | ✅ Active |
| < 2.0 | ❌ Not supported |

## Reporting a Vulnerability

**Please do NOT open a public GitHub issue for security vulnerabilities.**

Report security issues by emailing: **security@pecunovus.com**

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Your suggested fix (optional)

You will receive a response within **72 hours**. We follow responsible disclosure
and will coordinate a fix and public disclosure with you.

## Scope

In scope for security reports:
- Consensus mechanism vulnerabilities (PoT/PoS bypass, 51% attack vectors)
- Token contract bugs (double-spend, overflow, unauthorized mint/burn)
- RPC server vulnerabilities (authentication bypass, injection)
- Cryptographic weaknesses in SHA-512/VDF implementation
- Escrow logic flaws enabling unauthorized fund release
- Private key / wallet security issues

## License

This security policy applies to all code under the Apache 2.0 License in this repository.
