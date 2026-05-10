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

### Snap Store
- **Name**: `SNAPCRAFT_STORE_CREDENTIALS`
- **Get from**: Run `snapcraft export-login <file>` locally
- **Alternative**: https://snapcraft.io/account

### Chocolatey
- **Name**: `CHOCOLATEY_API_KEY`
- **Get from**: https://chocolatey.org/account
- **Note**: First submission requires manual review

### Homebrew Tap
- **Name**: `HOMEBREW_TAP_TOKEN`
- **Get from**: GitHub Personal Access Token with `repo` scope for `neul-labs/homebrew-tap`

### GitHub Token
- **Name**: `GITHUB_TOKEN` (automatic)
- **Permission**: Workflows have full access by default

### GPG Key (Optional - for DEB signing)
- **Name**: `GPG_KEY`
- **Name**: `GPG_PASSPHRASE`
- Used for signing DEB packages

## OIDC / Trusted Publishing Setup

### crates.io (Trusted Publishing — no long-lived token)

Instead of a `CARGO_REGISTRY_TOKEN`, we use OpenID Connect (OIDC) trusted publishing via `rust-lang/crates-io-auth-action`.

1. Publish the `gity` crate manually at least once (required before Trusted Publishing can be enabled).
2. Go to https://crates.io/crates/gity/settings and sign in as an owner.
3. Navigate to **Trusted Publishing**.
4. Add a new publisher:
   - **Repository**: `neul-labs/gity`
   - **Workflow**: `release.yml`
   - **Environment name**: `cargo` (optional but recommended)
5. Save.

The `release.yml` `publish-cargo` job uses `rust-lang/crates-io-auth-action@v1` to exchange the GitHub OIDC token for a short-lived crates.io upload token automatically.

### PyPI (Trusted Publishing — no long-lived token)

1. Go to https://pypi.org/manage/project/gity-cli/settings/publishing/
2. Click **Add a new pending publisher**
3. Fill in:
   - **Publisher**: GitHub Actions
   - **Repository**: `neul-labs/gity`
   - **Workflow**: `release.yml`
   - **Environment name**: `pypi`
4. Save

The `release.yml` `publish-pypi` job uses `pypa/gh-action-pypi-publish` with `id-token: write` permission, which exchanges the GitHub OIDC token for a short-lived PyPI upload token automatically.

### NPM (Trusted Publishing — no long-lived token)

npm supports OIDC trusted publishing for GitHub Actions.

1. The package must already exist on npmjs.com. Publish the first version manually:
   ```bash
   cd packages/npm
   npm publish --access public
   ```
2. Go to https://www.npmjs.com/package/gity-cli/access
3. Under **Trusted Publishers**, click **Add trusted publisher**
4. Fill in:
   - **Organization/User name**: `neul-labs`
   - **Repository name**: `gity`
   - **Workflow filename**: `release.yml`
   - **Environment name**: `npm` (optional but recommended)
5. Save

The `release.yml` `publish-npm` job uses `actions/setup-node@v4` with Node 24 and `npm publish --provenance --access public`. No `NPM_TOKEN` is required. Provenance attestations are generated automatically by npm when publishing via OIDC from a public repository.

Requirements:
- NPM account has 2FA enabled
- The package is public
- Node.js 24+ (ships with npm 11.5.1+)

## Crate Publish Order

The workspace crates must be published to crates.io in dependency order because they reference each other by version:

1. `gity-ipc`
2. `gity-git`
3. `gity-watch`
4. `gity-storage`
5. `gity-daemon`
6. `gity-cli`
7. `gity`

The release workflow handles this automatically.

## Artifact Attestations

Release binaries are attested using `actions/attest-build-provenance`. Users can verify the attestation on the GitHub release page or via the GitHub CLI:

```bash
gh attestation verify gity-0.2.0-x86_64-unknown-linux-gnu.tar.gz \
  --owner neul-labs
```

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

- [ ] crates.io Trusted Publisher configured (OIDC) for `gity`
- [ ] PyPI Trusted Publisher configured (OIDC)
- [ ] npm Trusted Publisher configured (OIDC) for `gity-cli`
- [ ] `SNAPCRAFT_STORE_CREDENTIALS` secret configured in GitHub
- [ ] `CHOCOLATEY_API_KEY` secret configured in GitHub
- [ ] `HOMEBREW_TAP_TOKEN` secret configured in GitHub
- [ ] Homebrew tap repository created at `neul-labs/homebrew-tap`
- [ ] Snap name registered
- [ ] NPM package name verified (`gity-cli`) and initial version published
- [ ] PyPI package name verified (`gity-cli`)
- [ ] Chocolatey package prepared
- [ ] All crate READMEs are SEO-optimized
- [ ] `CHANGELOG.md` updated for the new version

## Release Channels

| Platform | Channel | Auto-publish | Auth Method |
|----------|---------|--------------|-------------|
| crates.io | stable | Yes | OIDC Trusted Publishing |
| PyPI | stable | Yes | OIDC Trusted Publishing |
| NPM | latest | Yes | OIDC Trusted Publishing + Provenance |
| Snap Store | stable | Yes | Token (`SNAPCRAFT_STORE_CREDENTIALS`) |
| Chocolatey | approved | Manual first | Token (`CHOCOLATEY_API_KEY`) |
| Homebrew | stable | PR created | Token (`HOMEBREW_TAP_TOKEN`) |
| GitHub Releases | - | Yes | `GITHUB_TOKEN` + Attestations |

## Troubleshooting

### Version tag not created
- Ensure `crates/gity/Cargo.toml` version format is `"X.Y.Z"`
- Check CI workflow has permission to push tags (`contents: write`)
- Verify no existing tag with same version

### Package publish fails
- Verify package name is available
- Ensure no duplicate version published
- For crates.io OIDC: verify the Trusted Publisher is configured for the correct repository and workflow
- For PyPI OIDC: verify the Trusted Publisher is configured for the correct environment (`pypi`) and workflow (`release.yml`)
- For npm OIDC: ensure the package already exists on npmjs.com (first publish must be manual), the account has 2FA, and `repository.url` in `package.json` matches exactly (case-sensitive)

### Binary not found
- Check artifacts uploaded from `build` job
- Verify platform/architecture mapping
- Check artifact naming convention

### Cargo publish fails with dependency error
- Ensure crates are published in the correct order (see Crate Publish Order above)
- Verify all workspace dependency versions match the version being published
- Wait a few seconds between publishes for crates.io index propagation

### npm OIDC publish fails with E404
- The package must already exist on npmjs.com before OIDC publishing works
- Ensure Node.js 24+ is used (npm 11.5.1+ required)
- Check that the trusted publisher repository name matches exactly

## Post-Release Tasks

1. Update `CHANGELOG.md`
2. Verify attestations appear on the GitHub release
3. Verify npm provenance appears on https://www.npmjs.com/package/gity-cli
4. Verify crates.io versions show the "Published via GitHub Actions" badge
5. Create release announcement
6. Update documentation if needed
7. Monitor for issues on social media/platforms

## Support

For issues with package managers:
- **crates.io**: https://crates.io/support
- **PyPI**: https://pypi.org/support/
- **NPM**: https://www.npmjs.com/support
- **Snapcraft**: https://snapcraft.io/docs/contact-publisher
- **Chocolatey**: https://chocolatey.org/contact
