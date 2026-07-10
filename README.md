# mountOS Desktop

Desktop client for mountOS.

Built with Tauri 2, Svelte 5, and TypeScript.

## Development

Install dependencies and start the desktop application:

```sh
make install
make desktop-dev
```

Run the local verification suite:

```sh
make verify
```

Build the packaged desktop application:

```sh
make desktop-build
```

List every available command with `make help`.

## Packaging and signing

`make bundle` builds the distributable bundles for the current platform (macOS `.app` + `.dmg`, Windows NSIS installer). Signing and notarization read credentials from the environment, never from files in this repo.

### macOS

Export the signing identity before bundling so Tauri signs the `.app` and `.dmg` during the build, then notarize.

```sh
make release-macos
```

| Variable | Used by | Meaning |
|---|---|---|
| `APPLE_SIGNING_IDENTITY` | bundle, `sign-macos` | Developer ID Application identity name |
| `APPLE_ID` | `notarize-macos` | Apple ID email for notarytool |
| `APPLE_PASSWORD` | `notarize-macos` | App-specific password for that Apple ID |
| `APPLE_TEAM_ID` | `notarize-macos` | Apple Developer team identifier |

`make sign-macos` re-signs an already-built `.app` in place. After a re-sign, rebuild the `.dmg` or it ships the previously signed copy.

### Windows

Run from a POSIX shell (Git Bash or CI). `signtool` comes from the Windows SDK.

```sh
make release-windows
```

| Variable | Used by | Meaning |
|---|---|---|
| `WINDOWS_CERT_PFX` | `sign-windows` | Path to the code-signing certificate (.pfx) |
| `WINDOWS_CERT_PASSWORD` | `sign-windows` | Certificate password |
| `TIMESTAMP_URL` | `sign-windows` | RFC 3161 timestamp server, defaults to DigiCert |
