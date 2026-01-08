# Release Requirements

This document outlines the requirements and setup for publishing Gity to various package managers and distribution channels.

## Version Bump Process

1. Update version in `crates/gity/Cargo.toml`:
   ```toml
   [package]
   version = "0.2.0"  # Update this
   ```

2. Commit and push the changes:
   ```bash
   git add crates/gity/Cargo.toml
   git commit -m "chore: bump version to v0.2.0"
   git push origin main
   ```

3. The CI workflow will:
   - Detect the version change
   - Create and push a version tag (`v0.2.0`)
   - Trigger the release workflow
   - Build and publish to all platforms

## Required Secrets

Add these secrets to your GitHub repository (Settings > Secrets and variables > Actions):

### Cargo (crates.io)
- **Name**: `CARGO_REGISTRY_TOKEN`
- **Get from**: https://crates.io/settings/tokens
- **Permission**: Publish new versions

### PyPI
- **Name**: `PYPI_TOKEN`
- **Get from**: https://pypi.org/manage/account/token/
- **Scope**: Entire account (or specific project)

### NPM
- **Name**: `NPM_TOKEN`
- **Get from**: https://www.npmjs.com/settings/[username]/tokens
- **Type**: Automation

### Snap Store
- **Name**: `SNAPCRAFT_STORE_CREDENTIALS`
- **Get from**: Run `snapcraft export-login <file>` locally
- **Alternative**: https://snapcraft.io/account

### Chocolatey
- **Name**: `CHOCOLATEY_API_KEY`
- **Get from**: https://chocolatey.org/account
- **Note**: First submission requires manual review

### GitHub Token
- **Name**: `GITHUB_TOKEN` (automatic)
- **Permission**: Workflows have full access by default

### GPG Key (Optional - for DEB signing)
- **Name**: `GPG_KEY`
- **Name**: `GPG_PASSPHRASE`
- Used for signing DEB packages

## Manual Setup Required

### Homebrew Tap

1. Create the tap repository:
   ```bash
   # Create on GitHub: neul-labs/homebrew-tap
   gh repo create homebrew-tap --public --description "Homebrew tap for Gity"
   ```

2. Clone locally:
   ```bash
   gh repo clone neul-labs/homebrew-tap
   cd homebrew-tap
   ```

3. Create initial formula structure:
   ```bash
   mkdir Formula
   touch Formula/README.md
   git add .
   git commit -m "Initial commit"
   git push origin main
   ```

### Snap Name Registration

1. Register the snap name:
   - Visit https://snapcraft.io/register-name
   - Register `gity` as your snap name
   - Configure permissions (network, home-read)

### AUR Package

1. Create AUR account at https://aur.archlinux.org/

2. Submit PKGBUILD from `packages/arch/PKGBUILD`

## Release Checklist

Before first release, verify:

- [ ] All secrets configured in GitHub
- [ ] Homebrew tap repository created
- [ ] Snap name registered
- [ ] NPM package name verified (gity-cli)
- [ ] PyPI package name verified (gity)
- [ ] Chocolatey package prepared

## Release Channels

| Platform | Channel | Auto-publish |
|----------|---------|--------------|
| crates.io | stable | Yes |
| PyPI | stable | Yes |
| NPM | latest | Yes |
| Snap Store | stable | Yes |
| Chocolatey | approved | Manual first |
| Homebrew | stable | PR created |
| GitHub Releases | - | Yes |

## Troubleshooting

### Version tag not created
- Ensure `crates/gity/Cargo.toml` version format is `"X.Y.Z"`
- Check CI workflow has permission to push tags
- Verify no existing tag with same version

### Package publish fails
- Check token has required permissions
- Verify package name is available
- Ensure no duplicate version published

### Binary not found
- Check artifacts uploaded from `build` job
- Verify platform/architecture mapping
- Check artifact naming convention

## Post-Release Tasks

1. Update CHANGELOG.md
2. Create release announcement
3. Update documentation if needed
4. Monitor for issues on social media/platforms

## Support

For issues with package managers:
- **crates.io**: https://crates.io/support
- **PyPI**: https://pypi.org/support/
- **NPM**: https://www.npmjs.com/support
- **Snapcraft**: https://snapcraft.io/docs/contact-publisher
- **Chocolatey**: https://chocolatey.org/contact
