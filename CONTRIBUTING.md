# 🤝 Contributing to Auto TTS Desktop

Thank you for your interest in contributing to **Auto TTS Desktop**! This project follows **Google's Open Source Guidelines & Engineering Standards**.

---

## 📋 Code of Conduct

Please help maintain a respectful, welcoming, and productive environment. We expect all contributors to adhere to standard professional etiquette.

---

## 🛠️ Development Setup

### Prerequisites
1. **Node.js**: `v20.0.0+` and `npm`.
2. **Rust Toolchain**: `rustc` & `cargo` `1.90+` via [rustup](https://rustup.rs/).
3. **C++ Build Tools**: Microsoft Visual Studio C++ Build Tools or `LLVM`.

### Step-by-Step Build Instructions

```bash
# 1. Clone the repository
git clone https://github.com/your-username/auto-tts-desktop.git
cd auto-tts-desktop

# 2. Install Frontend Dependencies
npm install

# 3. Launch Application in Development Mode
npm run tauri dev
```

---

## 🧪 Testing Guidelines

Before submitting any Pull Request, ensure all quality verification checks pass cleanly:

```bash
# 1. Type-check Svelte & TypeScript
npm run check

# 2. Run Frontend Tests (Vitest + @testing-library/svelte)
npm run test

# 3. Run Rust Unit Tests (Text Chunker, Normalizer, Prompt Builder, Audio Merger, SQLite)
cd src-tauri
cargo test
```

### Writing Tests

- **Critical/High fixes must include regression tests** — no PR will be merged without at least 1 test protecting the fix.
- **Backend (Rust):** Use `#[test]` and `#[tokio::test]`. For SQLite concurrency tests, use file-backed `tempfile::tempdir()` DB instead of `:memory:`.
- **Frontend (Svelte/TS):** Use Vitest + `@testing-library/svelte`. Mock Tauri IPC via `vi.mock('@tauri-apps/api/core')`.

---

## 📐 Code Style Conventions

### Rust Backend
- Format code using `cargo fmt`.
- Address all warnings reported by `cargo clippy`.
- Keep error messages descriptive and return structured errors via `thiserror` or custom `AppError` enums.
- Use `#[serde(rename_all = "snake_case")]` for enums crossing IPC boundary.

### Svelte 5 Frontend
- Use Svelte 5 Runes (`$state`, `$derived`, `$effect`) for reactivity.
- Keep components modular and ensure accessible UI elements with proper ARIA attributes.
- No component should exceed 500 lines — extract sub-components as needed.

---

## 🔀 Git Commit Message Standards

We enforce Conventional Commits format:
- `feat: add vietnamese currency text normalizer`
- `fix: resolve exponential backoff jitter calculation`
- `docs: update system architecture diagram`
- `test: add unit test for SSML break tag parser`

---

## ✅ PR Checklist

Before requesting review, confirm:

- [ ] `npm run check` passes
- [ ] `npm run test` passes (or no frontend tests affected)
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] Critical/High fix includes regression test
- [ ] No hardcoded paths, magic strings, or `console.log` in committed code
- [ ] New IPC commands return `Result<T, AppError>`, not `Result<T, String>`

