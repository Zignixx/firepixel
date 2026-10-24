# Security Policy

Firepixel Studio parses complex image and document formats (FW-PNG, PSD, GIF, TIFF, SVG, PDF, AI/EPS and more), runs JSF
scripts and exposes a CLI and an MCP server (headless and as a bridge into the running app). Treat files, paths,
metadata, scripts and automation requests as potentially malicious.

## Reporting a vulnerability

Please do **not** open a public issue with a working exploit, a malicious sample file, secrets or crash dumps.
Use GitHub's **private vulnerability reporting** for this repository ("Security" tab ▸ "Report a vulnerability").
If that is not available, open a public issue that only says you have a security report and asks for a private
channel — without technical details.

Please give the maintainers a reasonable time to investigate and fix the problem before publishing details.

## What to include

- Firepixel version or commit, operating system and architecture;
- the affected component (file format, CLI command, MCP tool, JSF API, bridge);
- impact and the access an attacker needs;
- exact reproduction steps, expected and actual behaviour;
- crash logs or backtraces with personal paths and document content removed.

Malicious test files: first describe format, size, SHA-256 hash and observed effect; send the file only over the
agreed private channel. Never attach files taken from an installation of the reference application.

## Scope

- File parsing and decoding (decompression bombs, huge dimensions, integer overflow, memory exhaustion,
  recursion, hangs);
- import/export path handling (traversal, symlinks, unexpected overwrites);
- JSF scripting (`Files` API, escaping the intended file operations);
- the MCP server and the bridge: authentication, token handling, request validation;
- the AI model download (integrity checks) and release artifact integrity.

## Current design

- The MCP server and the bridge bind to `127.0.0.1` only, drop connections from other addresses, require a
  256-bit token on every request (constant-time comparison), check the `Host` header against DNS rebinding and run
  no shell commands. Token files are written to the user's local profile and removed when the server stops.
- Files are only touched through open, save, export and the JSF `Files` API.
- Downloaded AI models are verified by hash before use.
- The workspace forbids `unsafe` code; damaged files must produce errors, not panics.

Not yet implemented: per-client capabilities, request size and rate limits beyond the defaults of the HTTP stack,
and sandboxing of JSF file access.
