# Metadata and artwork

[Guide home](README.md)

Open **Settings → Library → Metadata & artwork**. Changes default to MusicOS-only overrides. Writing to music files is a separate choice for each application.

## Find an album

Use **Find albums** to search by album or artist. **All albums** includes releases without a suggestion. **Has suggestions** shows releases with matches ready to review. **Check library for suggestions** starts a background check. Cancel keeps the suggestions already found.

Open a release and choose **Cover**, **Details**, or **Songs**. Album and song menus take you straight to the relevant editor. On smaller windows, **Back to albums** returns you to the same list position.

## Review and save

- **Cover** compares the current image with a proposed cover. Choose a proposal if several are available. **Update cover only** shows how many songs will get the new cover.
- **Details** shows the current album values and any suggested changes. Choose the suggestions you want to use, or type your own values. Expand **See affected songs** when values differ across the album.
- **Songs** opens the tags or lyrics for one song. **More metadata tools → Bulk edit** is available for selections across albums.

Review the exact changes, then choose **Save in musicOS**. Use **Advanced · original file tags** in the review to write changes to the files as well. Check the recording and release sources when choosing between editions. If the comparison changes while you're editing, review the updated values before saving.

## History and Undo

History displays the latest 50 change batches across restarts. Undo restores the previous values, override protection, and relevant artwork references transactionally. Earlier records remain stored; this screen displays the latest 50.

Later conflicting edits or changed source identities prevent automatic Undo. Expand Compare changes to inspect before, after, and current values. No partially undone bulk batch is committed.

## Artist profiles

Artist profiles has its own cached/checking/unavailable status and retry action. Cached portraits and biographies remain available offline and during provider failures. Technical details stay behind Details rather than appearing in collection navigation.

## Ignore one match

Choose **Ignore suggestion** beside a suggestion or in its comparison. This hides that candidate across restarts and routine rescans, without changing any song fields or artwork. A genuinely different recording, release, or suggested value may still be offered.

The **Ignored** tab lists hidden matches. **Restore suggestion** allows the current matching candidate to be reviewed again. Changed songs or candidates still require a fresh comparison before applying.

## Also write to music files

This option starts off for every application. Review the affected files, supported fields, permissions and required backup space before enabling it. Pause playback before replacing files. MusicOS stages and validates format-aware tag changes without transcoding the audio. Unsupported or protected files fail with a specific error; a failed stage leaves originals untouched.

Original-file backups remain stored until you explicitly remove them under **History → Original-file backups**. File Undo checks both current metadata and file identities before restoring anything. Later changes stop automatic restoration rather than being overwritten. Removing a backup disables file Undo for that application.

Interrupted file work uses a recovery journal. MusicOS checks it at startup; **Retry interrupted file work** checks again after you resolve the reported access or identity conflict. Do not remove or edit journal files to force an operation through.
