# First-time setup and Windows releases

## 1. Understand the model

This project uses one long-lived branch, **main**. Short-lived `feature/...`,
`fix/...`, and `release/...` branches merge through pull requests. There is no
`develop` branch to keep synchronized. A release is an immutable tag on a commit
already merged into `main`, such as `v0.1.0`.

`Cargo.toml` holds `MAJOR.MINOR.PATCH`; the tag must be exactly `v` plus that
version. Increase PATCH for compatible fixes, MINOR for new functionality, and
MAJOR for incompatible changes. Before 1.0, document incompatible changes
explicitly. This workflow supports stable releases only, not prerelease suffixes.

| Event | Automatic result |
|---|---|
| Pull request or push to main | Windows format, tests, Clippy, workflow policy, dependency audit |
| Weekly schedule | Repeat CI, including fresh RustSec advisory checks |
| Push a vMAJOR.MINOR.PATCH tag | Validate version/ancestry, audit, test, build Windows x64, package |
| Successful tag build | Create a **draft** GitHub Release with artifacts |
| Maintainer publishes draft | Public release becomes available for users |

Builds run on `windows-2025` (a Windows Server CI image), targeting
`x86_64-pc-windows-msvc`. This produces Windows x64 binaries; it does **not** prove
Windows 11 UI compatibility. Test on a real Windows 11 machine or VM before
publishing. ARM64-native and 32-bit Windows are not release targets.

## 2. Create and connect the repository

The local folder has been initialized with an unborn `main` branch. No initial
commit, remote, GitHub repository, or protection rules have been created for you.
If starting from a fresh copy without `.git`, run `git init -b main` first.
Install Git for Windows and GitHub CLI, or use GitHub's website for remote setup.
Open PowerShell in the project root.

1. Review every source/document/example for private data and ownership. The
   historical design documents in `docs` are public too. `.gitignore` is not a
   secret scanner, and adding a path there does not untrack previously committed
   data. Never publish signing certificates or personal config.
2. Verify `LICENSE` and `Cargo.toml` reflect the approved GPL-3.0-only choice.
   GPL permits paid products; it does not impose a noncommercial restriction.
3. Configure your identity if needed, replacing the example values:

```powershell
git config user.name "Your Name"
git config user.email "your-public-or-github-noreply-address"
git add .
git diff --cached --stat
git diff --cached
git status --short
```

Check that `target`, `dist`, local configuration, and private files are absent.
If a secret has ever been committed, revoke/rotate it before publication;
removing it from the latest file does not erase history.

```powershell
git commit -m "Prepare App Launcher for public releases"
gh auth login
```

Create an **empty public** repository on GitHub (do not pre-create its README,
license, or gitignore), then use the exact remote URL GitHub supplies:

```powershell
git remote add origin https://github.com/YOUR-OWNER/YOUR-REPOSITORY.git
git push -u origin main
```

Replace `YOUR-OWNER/YOUR-REPOSITORY`; this is an example, not a configured remote.
If you prefer CLI creation after reviewing the staged contents:

```powershell
gh repo create YOUR-OWNER/YOUR-REPOSITORY --public --source . --remote origin --push
```

Use only one remote-creation method. Once connected, open **Actions** and confirm
the first `CI` run finishes successfully.

## 3. Protect publishing permissions in GitHub

These are server settings, not things a YAML file or local branch can enforce.

- **Settings > Actions > General**: enable Actions; use read-only workflow token
  permissions by default; keep approval of fork workflows enabled and do not
  expose secrets to fork PRs. Permit only required actions. The release
  publication job explicitly requests `contents: write`.
- **Settings > Rules > Rulesets** (or branch protection): target `main`; require
  pull requests, resolved discussions, and passing `Windows checks` and
  `Dependency audit` checks from `CI`. Select checks after their first run.
  Require review by another maintainer when one is available. Block force pushes
  and branch deletion; keep the bypass list as small as possible.
- Add a **tag ruleset** targeting `v*`: restrict tag creation to release
  maintainers and prohibit tag updates/deletion. A tag pushed by someone with
  sufficient access executes the workflow from that tag, so tag permissions are
  a security boundary. Do not repoint a released tag.
- **Settings > Code security**: enable Dependabot alerts, dependency graph,
  available secret scanning/push protection, and private vulnerability reporting.
  Dependabot PR schedules are supplied in `.github/dependabot.yml`.

The repository pins every action by full commit SHA. Review Dependabot action
updates, including what upstream commit is being trusted. Rust itself is pinned
in `rust-toolchain.toml`; update it through a normal reviewed PR. Audit and license
tools are also pinned by version and installed with locked dependency resolution.
They still execute third-party code, as do Cargo build scripts.

Never use `pull_request_target` to build PR code, and do not change a release job
to execute untrusted artifacts with a write token.

## 4. Make changes and prepare a version

Start with a clean checkout:

```powershell
git switch main
git pull --ff-only
git switch -c release/0.1.1
```

Edit `Cargo.toml` to `version = "0.1.1"`, update user documentation, then:

```powershell
cargo check
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
git add Cargo.toml Cargo.lock README.md
git diff --cached
git commit -m "Prepare version 0.1.1"
git push -u origin release/0.1.1
gh pr create --base main --title "Prepare version 0.1.1" --body "Version bump and release preparation."
```

Stage any other intentional documentation/code changes too. `cargo check`
refreshes the package version in `Cargo.lock`; review both together. Merge the PR
only after CI and review. For the very first release, use the existing `0.1.0`
version instead if that is the intended release.

## 5. Tag the merged commit

```powershell
git switch main
git pull --ff-only
git status --short
git tag -a v0.1.1 -m "App Launcher 0.1.1"
git push origin v0.1.1
```

The working tree should be clean before tagging. Push the tag explicitly; avoid
`git push --tags`, which can publish unrelated tags.

The release workflow verifies that the commit is already on `main` and the tag
matches `Cargo.toml`. It rebuilds from that checkout and uploads:

- `AppLauncher-Setup-VERSION.exe`: per-user installer, Windows 11 minimum.
- `AppLauncher-VERSION-windows-x64.zip`: portable executable, docs, examples,
  GPL text, original dependency notices, and generated dependency license inventory.
- `AppLauncher-VERSION-source.zip`: corresponding committed source, vendored
  Cargo dependencies, and an offline Cargo source configuration.
- `THIRD_PARTY_LICENSES.json` and `SHA256SUMS.txt`.

The full source bundle includes inactive-platform dependencies too, so it can be
substantially larger than the binary. Retain the source asset alongside binaries.

The publishing job downloads only this run's artifacts; it does not check out or
execute application code. It creates a draft, not an immediately public release.
No custom repository secrets or personal access token are required.

## 6. Review and publish the draft

Open the tag's **Actions** run, review logs (especially audit and license
warnings), then open the draft under **Releases**. Download artifacts from that
draft, not an unrelated local build.

On a clean Windows 11 x64 VM using a standard user:

- Verify file hashes against the accompanying checksum file.
- Install without an administrator prompt; confirm shortcuts and uninstall work.
- Start the portable executable normally. Check tray, hotkey, paging, config
  reload, and errors for missing programs.
- Launch `%SystemRoot%\System32\notepad.exe` with an argument containing spaces.
  Confirm bare `notepad.exe`, a relative `.exe`, `.cmd`/`.bat`, UNC executables,
  and network working directories are rejected.
- Test **Edit config** and malformed configuration. Existing users with bare
  executable names must migrate their config; see the README.
- Enable startup in Settings and sign out/in; `--background` should stay in the
  tray. Disable it and confirm the registry entry is removed.
- Explicitly run as administrator and confirm startup is refused with a message.
  Also verify ordinary double-click and non-elevated startup still succeed.
- Review `THIRD_PARTY_LICENSES.json` and the original files in
  `THIRD_PARTY_NOTICES`; the detector has `NOT FOUND` entries even for some
  packages with included license files. Resolve any genuinely missing notices.
  Check the
  corresponding source ZIP includes project files, `vendor`, and
  `.cargo\config.toml`; extract it and confirm an offline build with the Windows
  prerequisites works.

See [SECURITY.md](../SECURITY.md) for current dependency warning rationale.
Warnings are not automatically waived by a green audit: the default audit policy
blocks vulnerability-class advisories, not maintenance warnings.

Write release notes describing changes, migration requirements, Windows support,
and known issues. Do not describe the build as signed or externally audited.
Select **Publish release** only when review is complete. Users find downloads
under **Releases > Assets**; the app does not automatically update.

If a job fails, inspect the first failing step. A build failure can be re-run for
the same tag if the source is unchanged. If publication succeeded, re-running
publication intentionally fails rather than silently replacing release assets;
inspect the existing draft. For a source fix, merge a new version and create a
new tag. Never move a public tag or overwrite public binaries.

## 7. Build packages locally

Install the native Windows build prerequisites from the README, Inno Setup 6,
and the pinned license tool:

```powershell
cargo install cargo-bundle-licenses --version 5.0.0 --locked
cargo build --locked --release --target x86_64-pc-windows-msvc
.\scripts\package.ps1
```

Commit release inputs first. Packaging refuses a dirty checkout or an existing
`dist` directory to avoid mixing source or artifacts. Move the old `dist`
elsewhere before rebuilding. Failed partial output must also be moved before
retrying. Do not rely on an old executable after changing source: rebuild first.
The script checks every external tool's exit status and fails on errors.

## 8. Signing, trust, and maintenance

These executables and installer are **unsigned**. Windows SmartScreen may show
an unknown-publisher warning. Checksums alongside a download detect corruption
but do not authenticate the publisher if the repository/account is compromised.
Do not instruct users to disable security features globally.

Authenticode signing is a separate future setup requiring a publisher identity,
certificate or managed signing service, protected signing credentials, and
timestamping. It is not configured by these workflows. If added, sign the app
**before** packaging, sign the installer **after** compiling it, then compute
checksums from the final signed assets. Never store private keys in Git.

Review scheduled audit results and dependency PRs. Do not auto-merge dependency,
workflow, or toolchain updates solely because tests pass. Maintain private
vulnerability reporting and release maintainer access. GPL distribution and
third-party notice obligations apply to every release, including fixes.
