# Desktop releases and in-app updates

The updater supports Apple Silicon macOS apps, Windows x64 NSIS installs,
and Linux x64 AppImages. In a release build, open **Settings → General →
Software updates** to check GitHub Releases and choose **Download update**.
Once verified, the update stays ready until you choose **Restart to update**
or next launch musicOS. The app does not restart during playback, and it does
not check or download in the background. Development builds, browser
previews and other package formats are not updateable in-app.

## One-time setup

The updater checks [MusicOS Releases](https://github.com/HenryTheAddict/MusicOS/releases)
without credentials. The release workflow uses GitHub's built-in token to
create drafts in the same repository.

1. Run `npm run tauri -- signer generate -w /secure/path/musicos-updater.key`
   on a trusted machine. Back up the private key and password securely. Put
   the full contents of the `.pub` file in the [Actions variable](https://github.com/HenryTheAddict/MusicOS/settings/variables/actions)
   `MUSICOS_UPDATER_PUBLIC_KEY` and put the private key contents and password
   in the source repository's [Actions secrets](https://github.com/HenryTheAddict/MusicOS/settings/secrets/actions)
   as `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   Do not rotate that key or move the feed without an updater migration
   plan; already-installed copies trust the key and feed they shipped with.

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
   upload the full set to an unpublished MusicOS release.
3. Smoke-test the installers and update from an older signed build on each
   platform against a separate public test feed or a real release. Review
   the installers before publishing.
4. Publish the draft as a stable release and mark it as latest only when
   ready. Keep the installer, signed updater package, signature, and feed for
   each supported platform together. Existing installations without the
   updater need to install the first updater-enabled release manually.

GitHub's latest release exposes only the selected stable manifest. Tauri
rejects old/equal versions and verifies both the artifact and the version
bound into its signature before installing. For a bad published release,
publish a corrected build at a higher version rather than enabling downgrades.
