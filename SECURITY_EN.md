# 🛡️ Security Policy and Defensive Posture (`SECURITY.md`)

Security, user data integrity, and complete immunity against supply-chain attacks are core foundational pillars of **Rustext**.

---

## 📌 Supported Versions

| Version | Support Status | Third-Party Dependencies |
| :---: | :--- | :---: |
| **`0.2.x`** | ✅ **Actively supported** (Security patches and feature updates) | `0` (Pure `std::*`) |
| `< 0.2.0` | ⚠️ Upgrade to `v0.2.0` or later | `0` (Pure `std::*`) |

---

## 🚨 Reporting a Vulnerability

If you discover a security vulnerability in **Rustext**, please report it responsibly:

1. **Direct Contact**: Send a detailed email to **boleklolek** (`bolek.lolek@gmail.com`) or use the [**Security Advisories**](https://github.com/boleklolek/rustext/security/advisories) tab on GitHub.
2. **Information to Include**:
   - Technical description of the vulnerability and its estimated CWE classification.
   - Reproduction steps or a minimal test file (*Proof of Concept*).
   - Affected operating system and display backend (Wayland, X11, Win32, or TUI).
3. **Response Time**: We will acknowledge receipt of the report within **48 hours** and release a fix along with its assigned `SEC-XX` tracking ID and regression tests in `tests/gui_tests.rs`.

---

## 🔒 Security Architecture by Design

### 1. 0.0% Supply-Chain Attack Surface
Unlike modern editors that pull hundreds of `crates.io` packages or thousands of `npm` modules, **Rustext has zero external dependencies** in [`Cargo.toml`](Cargo.toml). This completely eradicates:
- *Typosquatting* and malicious packages in package registries.
- Transitive vulnerabilities from stale third-party code.
- Malicious third-party `build.rs` build scripts executing arbitrary host code during compilation.

### 2. Memory Safety in 100% Safe Rust
The core text buffer (*Piece Table*), software rasterizer (`Framebuffer`), TrueType SFNT decoder (`.ttf`), Flexbox engine, and LSP client are implemented in **Safe Rust**, structurally preventing buffer overflows, use-after-free, and heap corruption.

### 3. Safe Subprocess Execution & ReDoS Immunity
- **No Intermediate Shell Interpreters**: All invocations to external tools (`git`, LSP servers, formatters, and linters) pass vector argument lists directly to `std::process::Command::new()`, eliminating command injection via shell metacharacters (`CWE-78`).
- **ReDoS Immunity (`CWE-1333`)**: The search and syntax tokenizer engines operate in deterministic linear time $\mathcal{O}(N)$ without backtracking-vulnerable regular expressions.

---

## 📋 Historical OWASP / CWE Audit Log

| ID | Severity | Vulnerability / Threat Vector | CWE | Verified Mitigation |
| :---: | :---: | :--- | :---: | :--- |
| **SEC-01** | **Medium** | Untrusted local extension execution | [CWE-829](https://cwe.mitre.org/data/definitions/829.html) / [CWE-94](https://cwe.mitre.org/data/definitions/94.html) | Strict directory confinement to `~/.rustext/extensions` without dynamic arbitrary code execution. |
| **SEC-02** | **Medium** | Non-atomic file saving and truncation on disk full | [CWE-377](https://cwe.mitre.org/data/definitions/377.html) / [CWE-459](https://cwe.mitre.org/data/definitions/459.html) | Atomic writes using sibling temporary file, disk `sync_all()`, and `fs::rename()`. |
| **SEC-03** | **Low** | CLI argument injection in Wayland clipboard | [CWE-88](https://cwe.mitre.org/data/definitions/88.html) | Direct piping into `wl-copy` via `stdin`, avoiding `ARG_MAX` limits and flag injection. |
| **SEC-04** | **Low** | Infinite recursion from cyclic symlinks | [CWE-674](https://cwe.mitre.org/data/definitions/674.html) / [CWE-59](https://cwe.mitre.org/data/definitions/59.html) | `GlobalSearchEngine` skips symlinks and enforces a maximum depth of 32 levels. |
| **SEC-05** | **Low** | Draft deserialization path traversal | [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Strict confinement to `drafts/`, `is_safe_draft_path()` validation, and rejection of `..`. |
| **SEC-06** | **Info** | Terminal raw mode persistence upon panic | [CWE-755](https://cwe.mitre.org/data/definitions/755.html) | `std::panic::set_hook` automatically restores cursor visibility, main screen buffer, and `stty sane`. |
| **SEC-07** | **Low** | Predictable shared memory temporary file creation | [CWE-377](https://cwe.mitre.org/data/definitions/377.html) / [CWE-59](https://cwe.mitre.org/data/definitions/59.html) | Exclusive opening with `O_CREAT \| O_EXCL` and UNIX `0600` permissions in `/dev/shm`. |
| **SEC-08** | **Medium** | Path traversal in external `.theme` loading | [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Whitelist validation `is_safe_theme_name()` (max 64 bytes) and canonical `starts_with(canonical_base)` check. |
| **SEC-09** | **Low** | Denial of Service (DoS) via oversized `.theme` files | [CWE-400](https://cwe.mitre.org/data/definitions/400.html) / [CWE-776](https://cwe.mitre.org/data/definitions/776.html) | Strict limit `MAX_THEME_FILE_SIZE = 512 KB` and 256 files maximum per directory. |
| **SEC-10** | **Low** | Accumulative memory leak in Command Palette | [CWE-401](https://cwe.mitre.org/data/definitions/401.html) | Replaced `Box::leak` with `Cow<'static, str>` in `CommandItem`, freeing dynamic strings when closing the modal. |
