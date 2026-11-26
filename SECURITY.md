# Security policy

## Reporting a vulnerability
Do not post exploit details, private configuration, credentials, or personal paths in public issues. Use this repository's **Security > Advisories > Report a vulnerability** when private vulnerability reporting is enabled. If it is not available, ask the repository owner for a private reporting channel without disclosing the vulnerability publicly.

Maintainers: enable private vulnerability reporting before public launch. No response-time guarantee or external security certification is implied. Only the latest stable release is maintained. Update to the latest release before reporting an issue.

## Security controls

This is a user-controlled program launcher, not a sandbox, application allowlist,
or privilege broker. It has no network command listener, update downloader, script
engine, or elevation request. It uses `std::process::Command` with a separate
argument vector, not a constructed shell command. Explicitly configuring a shell
such as PowerShell is still possible: its arguments are executable code.

- **Least privilege**: The Windows app refuses an elevated process token **before** reading config or loading icons. Failure to inspect the token also stops startup. The installer installs per-user with `PrivilegesRequired=lowest`.
- **Full paths required for executable**: Executables require full drive-letter paths ending in `.exe`. Launch rejects
  relative names, PATH search, UNC/device paths, alternate data streams, scripts,
  and ambiguous dot/space components. It resolves paths and rejects UNC targets
  and mapped network drives before spawning. The editor uses the system directory
  returned by Windows rather than searching for `notepad.exe`.
- **Full paths required for icons and work dir**: Configured working directories and icons must also use full drive-letter
  paths. Icons are still parsed by image decoders or Windows Shell; they are not
  safe untrusted input. Resource limits reduce, but do not eliminate, parser risk.
- `%APPDATA%\AppLauncher\config.yaml`, its directory, the executable directory, configured executables/icons, working directories, environment variables, and DLL search locations are trusted **per-user** state. Keep them out of shared writable folders. Do not copy configurations from unknown sources.
- Another process running as the same user can replace this state or executable, race path checks, or launch programs directly. Canonicalization does not create an atomic filesystem security boundary. A drive-letter mapping alone also does not establish filesystem trust. The app does not defend an already-compromised account, administrator, kernel, or malicious installed Shell extension.
- Child processes normally inherit the launcher's standard-user token. Windows itself or a child application can still offer UAC elevation. This launcher cannot guarantee that a chosen child program will never elevate.

Do not use compatibility settings or scheduled tasks that run this app as
administrator. Double-click and ordinary sign-in startup require no special setup.

## Release controls
Action implementations are pinned to immutable commits. Build jobs have a
read-only token and do not persist checkout credentials. The separate release publication job gets write access but never executes the built executable or checks out application code. This reduces exposure; it does not protect against a malicious maintainer, compromised dependency/compiler, or improperly configured repository permissions. See [release setup](docs/RELEASING.md) for branch/tag rules, dependency review, and signing limitations.
