# Releasing Mneme

Releases are built by GitHub Actions and published to
[GitHub Releases](../../releases). Binaries are currently **unsigned** (see the
install notes in the README).

## Cut a release

1. **Bump the version** in both files (keep them in sync):
   - `package.json` → `"version"`
   - `apps/desktop/src-tauri/tauri.conf.json` → `"version"`

2. **Commit** the bump:
   ```bash
   git add package.json apps/desktop/src-tauri/tauri.conf.json
   git commit -m "chore: release v0.1.0"
   ```

3. **Tag and push** (the tag must start with `v`):
   ```bash
   git tag v0.1.0
   git push origin main
   git push origin v0.1.0
   ```

4. **Wait for CI.** The `Release` workflow builds macOS (universal), Windows, and
   Linux in parallel and creates a **draft** GitHub Release with all artifacts
   attached:
   - macOS: `.dmg`, `.app.tar.gz`
   - Windows: `.msi`, `.exe` (NSIS)
   - Linux: `.AppImage`, `.deb`

5. **Review and publish.** Open the draft release, edit the notes, verify every
   artifact is attached, then click **Publish release**.

## Enabling code signing later

The release workflow (`.github/workflows/release.yml`) has a commented `env`
block. To ship signed builds, add the matching values as repository **Secrets**
and uncomment the relevant lines — no other change is required.
