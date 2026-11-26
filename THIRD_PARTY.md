# Third-party software

Open WinApp Launcher is licensed under **GPL-3.0-only**, at the repository owner's choice.
Commercial use and selling copies are allowed under that license. Distribution
of covered binaries and derivatives carries GPL source and licensing obligations.
See `LICENSE` for the full terms.

Slint offers a GPLv3 licensing option; this project uses that option rather than
claiming a proprietary license. Other dependencies retain their own licenses.
The exact versions are recorded in `Cargo.lock`.

The packaging script generates `THIRD_PARTY_LICENSES.json`, bundles it with both
binary distributions, and creates a matching source ZIP containing the committed
project plus vendored dependency source and an offline Cargo configuration.
The inventory includes inactive-platform and build dependencies, so it is
conservative rather than a precise list of code linked into the executable.

License detection does not find every packaged notice: the current inventory
contains `NOT FOUND` entries, including Slint and CC0/Boost-licensed packages.
Therefore packaging also copies original `LICENSE`, `COPYING`, `NOTICE`,
`AUTHORS`, and related files from vendored crates into `THIRD_PARTY_NOTICES`,
preserving their dependency-relative locations. This directory is included in
both the portable ZIP and installation. `GPL-3.0-only.txt` supplies the GPL text
for Slint's selected GPL option. The inventory's `NOT FOUND` marker does not mean
those original notices are absent; inspect the actual files and corresponding
source when reviewing.

**Before publishing a draft release**, review license-tool warnings and the
generated inventory for missing license text, incorrect detection, attribution,
and redistribution requirements. Automatic discovery cannot guarantee license
compliance. If a license is missing, add the verified upstream notice/license to
the repository and update packaging before rebuilding. Do not publish incomplete
notices. Tool installation dependencies (for example the packaging/audit tools)
are not part of the application binary.

Keep the matching source ZIP available for as long as binaries are offered.
Mirror it together with binaries when redistributing outside GitHub; do not
distribute only an executable or promise source will be supplied later.
The source ZIP can be built with the Windows prerequisites in `README.md` using:

```powershell
cargo build --offline --locked --release --target x86_64-pc-windows-msvc
```

Rust, the Windows SDK, Visual Studio Build Tools, and Inno Setup are separately
installed tools, not included in the source ZIP. Consult the license terms and
legal advice if distribution obligations are unclear.
