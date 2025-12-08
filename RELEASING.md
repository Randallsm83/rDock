# Release Process

This document describes how to create a new release of rdock.

## Automatic Releases via GitHub Actions

The project uses GitHub Actions to automatically build and publish releases when a new tag is pushed.

### Creating a New Release

1. **Update version in Cargo.toml** (if needed)
   ```toml
   [package]
   version = "0.2.0"  # Update this
   ```

2. **Commit any changes**
   ```bash
   git add .
   git commit -m "Prepare v0.2.0 release"
   git push
   ```

3. **Create and push a tag**
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. **GitHub Actions will automatically:**
   - Build the release binary
   - Create a ZIP archive with `rdock.exe`, `config.toml`, and `README.md`
   - Create a GitHub Release with the archive attached
   - Name it `rdock-v0.2.0-windows-x64.zip`

5. **Verify the release:**
   - Go to https://github.com/Randallsm83/rdock/releases
   - Check that the new release appears with the ZIP file
   - Download and test the binary

## Manual Release (if needed)

If you need to create a release manually:

```bash
# Build release
cargo build --release

# Create release directory
mkdir release
Copy-Item target/release/rdock.exe release/
Copy-Item config.toml release/
Copy-Item README.md release/

# Create archive
Compress-Archive -Path release/* -DestinationPath rdock-v0.2.0-windows-x64.zip

# Upload to GitHub Releases manually
```

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR** (v1.0.0): Breaking changes
- **MINOR** (v0.2.0): New features, backward compatible
- **PATCH** (v0.1.1): Bug fixes, backward compatible

## Checklist

Before releasing:
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] README is up to date
- [ ] CHANGELOG is updated (if you create one)
- [ ] Version in Cargo.toml matches tag

## Future: Package Managers

Once the project is more established, consider submitting to:

### Scoop
Create a manifest in a scoop bucket:
```json
{
    "version": "0.2.0",
    "url": "https://github.com/Randallsm83/rdock/releases/download/v0.2.0/rdock-v0.2.0-windows-x64.zip",
    "bin": "rdock.exe",
    "checkver": "github",
    "autoupdate": {
        "url": "https://github.com/Randallsm83/rdock/releases/download/v$version/rdock-v$version-windows-x64.zip"
    }
}
```

### Winget
Submit to [winget-pkgs repository](https://github.com/microsoft/winget-pkgs)

### Chocolatey
Create a `.nuspec` file and publish to [Chocolatey.org](https://chocolatey.org/)
