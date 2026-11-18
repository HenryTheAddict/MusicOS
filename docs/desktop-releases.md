# Desktop releases and in-app updates

The updater supports Apple Silicon macOS apps, Windows x64 NSIS installs,
and Linux x64 AppImages. In a release build, open **Settings → General →
Software updates** to check GitHub Releases and choose **Install & restart**.
There are no background checks or automatic installs. Development builds,
browser previews and other package formats are not updateable in-app.

## One-time setup

1. Use an initialized GitHub repository for distributing builds, with no
   private source history. The direct GitHub Releases feed requires public
   read access; do not change the source repository's visibility.
2. In the source repository's [Actions variables](https://github.com/HenryTheAddict/MusicOS/settings/variables/actions),
   set `MUSICOS_RELEASE_REPOSITORY` to the distribution repository's
   `OWNER/REPO`.
3. Run `npm run tauri -- signer generate -w /secure/path/musicos-updater.key`
   on a trusted machine. Back up the private key and password securely. Put
   the full contents of the `.pub` file in the Actions variable
   `MUSICOS_UPDATER_PUBLIC_KEY` and put the private key contents and password
   in the source repository's [Actions secrets](https://github.com/HenryTheAddict/MusicOS/settings/secrets/actions)
   as `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   Do not rotate that key or move the feed without an updater migration
   plan; already-installed copies trust the key and feed they shipped with.
4. To allow the workflow to create a draft in the distribution repository,
   add a fine-grained token restricted to that repository with **Contents:
   Read and write** as the `MUSICOS_RELEASE_TOKEN` Actions secret. No client
   receives this token. You can also attach the verified workflow artifact
   to a release manually instead.

The updater signature is separate from OS code signing. For normal Mac
distribution, configure `APPLE_CERTIFICATE` (base64 Developer ID Application
certificate), `APPLE_CERTIFICATE_PASSWORD`, optional `APPLE_SIGNING_IDENTITY`,
and `APPLE_ID`, `APPLE_PASSWORD` (app-specific), and `APPLE_TEAM_ID` for
notarization. Without those, an ad-hoc build is intended for internal testing.
Windows builds are updater-signed; Authenticode is not configured here.

## Prepare and publish

1. Increase `version` in `product.config.json`, run
   `npm run product:sync` and `cargo metadata --format-version 1 >/dev/null`,
   and land the version and lockfile changes on the default branch. The
   workflow derives the release tag and URLs from that version.
2. Run [Desktop release](https://github.com/HenryTheAddict/MusicOS/actions/workflows/desktop-release.yml)
   on the default branch. It downloads the existing checksum-pinned Sparklist
   evaluation model, builds native bundles, verifies each package signature
   against the update public key, and assembles `latest.json`. By default it
   only stores a verified Actions artifact. Select `create_draft` to also
   upload the full set to an unpublished release in the distribution repo.
3. Smoke-test the installers and update from an older signed build on each
   platform against a separate test feed or a real, approved release. Review
   that the bundles and bundled assets are approved for distribution. **The
   Sparklist evaluation candidate is currently review-only; do not publish
   packages containing it until it is approved for distribution.**
4. Publish the draft as a stable release and mark it as latest only when
   ready. Keep the installer, signed updater package, signature, and feed for
   each supported platform together. Existing installations without the
   updater need to install the first updater-enabled release manually.

GitHub's latest release exposes only the selected stable manifest. Tauri
rejects old/equal versions and verifies both the artifact and the version
bound into its signature before installing. For a bad published release,
publish a corrected build at a higher version rather than enabling downgrades.
