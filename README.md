# Castellum

> **Castellum** (*lat. fortress, stronghold*) — a zero-knowledge, memory-hardened terminal password manager and cryptographic vault engine written in Rust for Linux.

---

## Highlights

* **True Plausible Deniability**: Binary container format indistinguishable from cryptographic white noise (zero plaintext magic headers or metadata leaks).
* **Sole AEAD Standard**: Pure XChaCha20-Poly1305 authenticated encryption with 192-bit nonces (constant-time, side-channel immune).
* **Key Derivation & Separation**: Argon2id (RFC 9106) coupled with HKDF-SHA512 subkey splitting.
* **Linux Kernel Hardening**:
  * `memfd_secret` page unmapping from kernel tables.
  * `mlockall(MCL_CURRENT | MCL_FUTURE)` to prohibit secret eviction to SWAP.
  * `prctl(PR_SET_DUMPABLE, 0)` anti-debugging & anti-ptrace enforcement.
  * `madvise(MADV_DONTDUMP)` core dump exclusion.
  * `seccomp` / `landlock` network socket dropping (zero telemetry guarantee).
* **Linux Clipboard Protection**: Native integration with `x-kde-passwordManagerHint = "secret"` to inhibit clipboard history snooping, plus direct TTY piping.
* **Modern TUI & Headless CLI**: Fast terminal user interface powered by `ratatui` with interactive Command Palette (`Ctrl+P`), paired with `clap` v4 for scripting.

---

## Status

Active development.
