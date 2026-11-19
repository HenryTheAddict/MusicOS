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
errors, missing-storage read/stat/open failures (ENOENT/EIO/ENODEV/ENXIO), or changed
root identity. Indexing failures remain sticky through reconciliation, including
metadata reads when a managed content hash is already cached. Malformed metadata
still uses the normal fallback rather than implying missing storage.
Linux identity includes device, inode and mount
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
| Root removed partway through indexing | Rows retained; source unavailable |
| Files removed during indexing, root unchanged | Rows retained; source unavailable |
| Cached hash with missing source file | Read failure propagated instead of hidden |
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

Latest full default tests: 45 library, 6 CLI, 2 offscreen, 21 product-flow and 8
resilience tests passed (82 total). The new indexing-phase regression test removes
either the whole root or only its files after indexing starts, leaving enough
files for subsequent parallel batches to encounter read failures. Unit tests
cover missing-storage error classification and cached-hash reads. Latest host
output is in `out/resilience-checks/host-followup-test.log`.
The earlier embedded feature combination compiled on this host;
it was not executed on device. Earlier Clippy completed successfully with the same 77
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

## QEMU results

The aarch64 Buildroot image and kernel were rebuilt using the tracked QEMU
configuration (no scratch-kernel override):

```sh
sudo apt-get install -y qemu-system-arm bison flex
env -u BASH_ENV scripts/build-qemu.sh
# On an existing output tree, regenerate the kernel config and resync app sources:
env -u BASH_ENV make -C vendor/buildroot O="$PWD/out/qemu" \
  linux-reconfigure musicos-init-rebuild
env -u BASH_ENV scripts/build-qemu.sh
python3 scripts/verify-qemu-resilience.py out/resilience-checks/qemu/fresh-run
```

The QEMU fragment now explicitly enables `NLS_CODEPAGE_437` and `NLS_ISO8859_1`
alongside the existing FAT/VFAT settings. All four resolved symbols are `y`.
This fixes the earlier `FAT-fs (sda): codepage cp437 not found` mount failure.
Rebuild logs are `out/resilience-checks/qemu/tracked-rebuild.log` and
`out/resilience-checks/qemu/final-rebuild.log`.

Choose a fresh output directory. The verifier retains all disposable images,
QEMU command lines, serial logs, API snapshots, diagnostics and `results.json`.
It requires passwordless sudo for host loop mounts, uses Buildroot's matching
`resize2fs`, and never attaches real devices. The static scratch init wrapper
mounts fixture filesystems, prints observed diagnostics/temp-file existence,
and execs the unchanged aarch64 `/sbin/init` as PID 1. HTTP requests exercise the
real update, library-scan and app-startup code; QMP controls unplugging. No guest
shell, Rust test executable, or implementation substitution is used. Playback
and framebuffer output are not checked.

The accepted full verifier run exited 0. All six scenarios passed using the
tracked kernel and latest application. Evidence is under
`out/resilience-checks/qemu/tracked-final/`, with console output in
`out/resilience-checks/qemu/tracked-final.log`:

| Scenario | Result |
| --- | --- |
| x86_64 ELF on aarch64 | PASS: `ELF architecture mismatch: expected machine 183, got 62` |
| ELF truncated before program header table | PASS: `truncated ELF program header table` |
| Truncated SquashFS | PASS: `truncated or invalid SquashFS bytes_used` |
| Interrupted valid install/reboot | PASS: seeded leftovers cleaned, hard-killed active install recovered, new executable booted |
| USB removed during active reference scan | PASS: all 2,000 row IDs retained; source becomes unavailable |
| Read-only 8 MiB ext4 DB disk | PASS: failed persistent open logged, memory fallback started, HTTP diagnostics remained `ok` |

Each malformed case preserves both `/sbin/init` and an existing pending `.next`,
and retains a stage-failure `last-update-result`. The original executable SHA-256
is `81d99808e6d657bc5e4f3e2d591ce7644c4bdf98be98a0923176db455df5c666`.
The read-only case logs
`failed to open sqlite DB at /mnt/fault/library.sqlite3` and
`library persistence unavailable; using a temporary in-memory database`.
This verifies failed-open fallback, not a write against an already-open database
or SQLite abrupt-restart durability (PR #3).

The verifier seeds `.tmp` and `.installing` leftovers and a valid 64 MiB `.next`
(the real aarch64 executable with zero padding). The first boot logs both
`removed interrupted update leftover` paths. Only after a newly growing
`.installing` exceeds 1 MiB does the host wait six seconds and kill QEMU, with
root-disk writes throttled to 2 MiB/s. Serial evidence includes
`RESILIENCE installing exists`. The next boot installs the pending
update; a further boot executes the newly installed padded binary and returns
HTTP diagnostics `status: ok`. Offline SHA-256 checks verify the installed
binary, original backup and absence of leftovers. In this cut, the newly created
partial `.installing` did not survive filesystem recovery; its removal on that
reboot is therefore not claimed. Explicit seeded-file cleanup is independently
demonstrated by the first boot. Earlier harness-development runs are retained
but are not the accepted interruption evidence.

The final unplug test captures an active full reference scan of `/mnt/usb/sda`
with 2,000 discovered files and 184 processed when QMP removes `usb-storage`.
Serial logs show
`usb 2-1: USB disconnect, device number 2`, followed by per-file indexing errors.
The completed source API reports `available: false` with a storage error;
offline JSON comparison confirms the identical 2,000 row IDs before and after.
Enumeration had completed before unplugging, so this exercises the newly fixed
indexing-phase path rather than only traversal-error handling. The earlier
failure (source incorrectly available) remains captured in `run-5/` and `run-7/`.

Use repeatable `--scenario` options to recheck only affected scenarios, e.g.:

```sh
python3 scripts/verify-qemu-resilience.py \
  --scenario interrupted-install --scenario removed-mid-scan \
  out/resilience-checks/qemu/fresh-selected-run
```

## H700 pending

The separate H700 configuration is
`buildroot-external/configs/musicos_rg35xx_defconfig`, using Linux 7.0.12's arm64
default config plus `board/rg35xx/linux.fragment`. The fragment enables FAT/VFAT
but does not explicitly set the NLS tables. Unlike QEMU's minimal base, that
kernel's `arch/arm64/configs/defconfig` already enables both tables, and the H700
fragment does not disable them. No equivalent input-configuration gap was found.
H700 files were left unchanged; no H700 kernel build or physical boot was done.

- Real USB unplug during enumeration, copy and playback; verify
  preserved library records and diagnostic logging.
- Physical SD power cuts during copy, backup, rename, directory sync and DB writes;
  assess filesystem recovery and PR #3's SQLite durability separately.
- Install an actual signed/release-controlled Knulli SquashFS and verify boot,
  hardware operation and manual restoration from `.previous`.
