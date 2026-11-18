# Embedded update and storage resilience

## Scope

USB updates validate ELF64 little-endian headers against the compiled host/device
architecture, executable type, header size and program-header table bounds.
SquashFS requires a complete v4 superblock, a power-of-two 4 KiB–1 MiB block size,
and bytes_used within the file. Validation runs before staging, after copying,
and before replacing the installed target. These checks detect malformed headers
and truncation; they are **not signatures, checksums or a bootability guarantee**.

Staging and installation use synced temporary files followed by rename. Boot
cleanup removes known `.tmp`/`.installing` leftovers. Existing targets require a
successfully copied and synced `.previous` backup before replacement. Directory
sync errors propagate. Copy failures remove partial copies before recording a
diagnostic, freeing space when a staging filesystem fills. Package attempts log
and write `<staging>/last-update-result` JSON (timestamp, package, operation,
outcome, reason); diagnostic persistence itself is best-effort and logs errors
when storage is unavailable. Backups do not provide automatic rollback or A/B boot.

Reference scans preserve unseen rows and mark the source unavailable on traversal
errors or changed root identity. Linux identity includes device, inode and mount
ID; it is checked again immediately before reconciliation, after indexing. This
does not eliminate a removal race *during* deletion. USB-only indexing does not
delete unseen rows. A failed persistent DB open logs its path and uses an
independent initialized in-memory DB; those changes are volatile. An existing
read-only DB may open successfully: later writes return errors rather than
automatically switching databases. No rendering, playback or SQLite durability
policy changes are included; `database.rs` and `server.rs` remain owned by PR #3.

## Reproduction

```sh
cargo test -p musicos-init --test update_storage_resilience -- --nocapture
scripts/verify-update-storage-resilience.sh
cargo test -p musicos-init
cargo clippy -p musicos-init --all-targets
cargo test -p musicos-init --features pid1,device-audio --no-run
cargo fmt --check
```

The script builds the scenario test executable, runs directory-based checks, then
uses passwordless sudo if available to create disposable 192 MiB vfat USB and
8 MiB ext4 storage/staging loop images. It never touches real block devices.
Mounts and files are cleaned up on exit. Without sudo it explicitly skips the
loop-image cases and retains directory-based validation/fallback checks.

## Host results

Verified on Ubuntu x86_64 with passwordless sudo:

| Scenario | Result |
| --- | --- |
| Wrong architecture/class, truncated ELF program table | Rejected; existing target and pending update unchanged |
| Invalid SquashFS version/block size/bytes_used | Rejected with diagnostic |
| Stale partial temp files and invalid pending update | Boot cleanup; target intact; failure diagnostic |
| Backup creation failure | Install rejected; original retained |
| Valid ELF and SquashFS header fixtures | Installed with success diagnostic |
| Removed root after enumeration | Rows retained; source unavailable |
| Unreadable subtree | Unseen rows retained; source unavailable |
| Unopenable persistent DB | App starts; independent memory DBs accept writes |
| Read-only SQLite connection/filesystem | App starts; attempted write returns readonly error |
| Lazy USB unmount while stage copy is active | Complete validated copy finishes and installs |
| Full staging ext4 image | ENOSPC surfaced; original retained; diagnostic written |
| Full SQLite ext4 image | SQLite disk-full error returned |

Lazy unmount detaches the namespace but leaves open descriptors valid. Its
successful copy is **not evidence of handling physical unplug I/O failures**.
Header fixtures are synthetic, not bootable release images. No UI visual check
was performed.

Full default tests: 43 library, 6 CLI, 2 offscreen, 21 product-flow and 7 resilience
tests passed (79 total). The embedded feature combination compiled on this host;
it was not executed on device. Clippy completed successfully with the same 77
printed warning diagnostics as origin/main (no new warnings). Workspace
`cargo fmt --check` fails on preexisting formatting: both this branch and
origin/main report 845 diff blocks. Unrelated formatting was intentionally left
untouched. Check logs are in local ignored `out/resilience-checks/`.

Before-fix evidence used a detached worktree at origin/main `3d19408` with only
the new test harness copied in. Individually running the relevant test functions
produced:

```text
malformed_packages_preserve_target_and_pending_update:
assertion failed: usb::get_update_from_usb().is_err()
removed_root_during_scan_preserves_rows_and_marks_unavailable:
assertion failed: !db.library_source(source.id).unwrap().unwrap().available
unopenable_persistent_database_uses_independent_memory_databases:
called `Result::unwrap()` on an `Err` value: No such file or directory (os error 2)
```

Each individual baseline test reported `FAILED. 0 passed; 1 failed`.

## Pending target checks

- QEMU aarch64 image build/boot (`env -u BASH_ENV scripts/build-qemu.sh`), then
  repeat staged update/reboot and storage-error checks. No Buildroot build started.
- Physical H700: real USB unplug during enumeration, copy and playback; verify
  preserved library records and diagnostic logging.
- Physical SD power cuts during copy, backup, rename, directory sync and DB writes;
  assess filesystem recovery and PR #3's SQLite durability separately.
- Install an actual signed/release-controlled Knulli SquashFS and verify boot,
  hardware operation and manual restoration from `.previous`.
