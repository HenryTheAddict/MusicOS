#!/usr/bin/env python3
"""Exercise the actual aarch64 PID 1 using disposable disks, HTTP and QMP.

Requires a completed build-qemu.sh build, passwordless sudo and e2fsprogs/dosfstools.
The scratch static init wrapper only mounts fixture disks and reports observations;
it execs the unmodified musicos-init. Never attach real block devices here.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import socket
import struct
import subprocess
import time
from urllib.error import URLError
from urllib.request import Request, urlopen
import wave


ROOT = Path(__file__).resolve().parents[1]
WRAPPER = r'''
#include <sys/mount.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>
extern char **environ;
int main(void) {
    mkdir("/proc",0755); mkdir("/sys",0755); mkdir("/dev",0755);
    mount("proc","/proc","proc",0,0); mount("sysfs","/sys","sysfs",0,0);
    mount("devtmpfs","/dev","devtmpfs",0,0);
    mkdir("/mnt",0755); mkdir("/mnt/usb",0755); mkdir("/mnt/usb/sda",0755);
    mkdir("/mnt/fault",0755);
    const char *device = getenv("RESILIENCE_DISK");
    if (device) {
        const char *fs = getenv("RESILIENCE_FS");
        const char *where = !strcmp(fs,"vfat") ? "/mnt/usb/sda" : "/mnt/fault";
        int ok = -1;
        for (int i=0;i<200 && ok;i++) {
            ok = mount(device,where,fs,getenv("RESILIENCE_RO")?MS_RDONLY:0,0);
            if (ok) usleep(50000);
        }
        fprintf(stderr,"RESILIENCE mount %s at %s: %s\n",device,where,ok?strerror(errno):"ok");
        if (ok) return 1;
    }
    if (!fork()) {
        char old[4096]="", text[4096], path[512];
        const char *stage=getenv("MUSICOS_UPDATE_STAGING");
        if (!stage) stage="/var/lib/musicos/update";
        snprintf(path,sizeof(path),"%s/last-update-result",stage);
        int reported=0;
        for (;;) {
            FILE *f=fopen(path,"r");
            if(f) {
                size_t n=fread(text,1,sizeof(text)-1,f); text[n]=0; fclose(f);
                if(strcmp(old,text)) { fprintf(stderr,"RESILIENCE diagnostic %s\n",text); strcpy(old,text); }
            }
            struct stat s;
            if(!reported && !stat("/sbin/.init.installing",&s) && s.st_size>1048576) {
                fprintf(stderr,"RESILIENCE installing exists size=%lld\n",(long long)s.st_size);
                reported=1;
            }
            usleep(20000);
        }
    }
    char *args[]={"/sbin/init",NULL};
    execve(args[0],args,environ); perror("exec musicos-init"); return 1;
}
'''


def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def wait(label, read, ready=bool, timeout=120):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            last = read()
            if ready(last):
                return last
        except (URLError, TimeoutError, ConnectionError, FileNotFoundError) as error:
            last = str(error)
        time.sleep(0.2)
    raise RuntimeError(f"{label} timed out: {last!r}")


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


@contextmanager
def mounted(image, directory, fs="ext4"):
    directory.mkdir()
    options = "loop" + (f",uid={os.getuid()},gid={os.getgid()}" if fs == "vfat" else "")
    run("sudo", "-n", "mount", "-t", fs, "-o", options, image, directory)
    try:
        yield directory
    finally:
        run("sudo", "-n", "umount", directory)
        directory.rmdir()


class Guest:
    def __init__(self, args, name, disk, fixture=None, fs="vfat", env=None, throttle=False):
        self.args, self.name = args, name
        self.serial = args.output / f"{name}-serial.log"
        self.qmp_path = args.output / f"{name}.sock"
        cmdline = ("console=ttyAMA0 root=/dev/vda rw rootwait init=/sbin/resilience-init "
                   "MUSICOS_RESOLUTION=640x480 "
                   "ip=10.0.2.15::10.0.2.2:255.255.255.0::eth0:off ")
        values = {"MUSICOS_USB_ROOTS": "/mnt/usb/sda",
                  "MUSICOS_UPDATE_STAGING": "/var/lib/musicos/update"}
        if fixture:
            values.update(RESILIENCE_DISK="/dev/sda", RESILIENCE_FS=fs)
        values.update(env or {})
        cmdline += " ".join(f"{key}={value}" for key, value in values.items())
        command = ["qemu-system-aarch64", "-M", "virt", "-cpu", "cortex-a53", "-smp", "4",
                   "-m", "512M", "-kernel", str(args.kernel or args.images / "Image"),
                   "-drive", f"file={disk},if=none,format=raw,id=root,cache=writethrough" +
                   (",bps_wr=2097152" if throttle else ""),
                   "-device", "virtio-blk-pci,drive=root", "-device", "virtio-gpu-pci",
                   "-device", "virtio-keyboard-pci", "-device", "virtio-tablet-pci",
                   "-display", "none", "-monitor", "none", "-no-reboot",
                   "-serial", f"file:{self.serial}", "-qmp", f"unix:{self.qmp_path},server=on,wait=off",
                   "-netdev", f"user,id=net0,hostfwd=tcp:127.0.0.1:{args.port}-:8080",
                   "-device", "virtio-net-pci,netdev=net0", "-append", cmdline]
        if fixture:
            command += ["-device", "qemu-xhci,id=xhci", "-drive",
                        f"file={fixture},if=none,format=raw,id=usb,cache=writethrough",
                        "-device", "usb-storage,id=removable,drive=usb"]
        (args.output / f"{name}-command.txt").write_text(shlex.join(command) + "\n")
        self.log = (args.output / f"{name}-qemu.log").open("wb")
        self.process = subprocess.Popen(command, stdout=self.log, stderr=self.log)

    def __enter__(self):
        try:
            wait("QMP socket", self.qmp_path.is_socket, timeout=10)
            self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self.socket.settimeout(20)
            self.socket.connect(str(self.qmp_path))
            self.stream = self.socket.makefile("rwb", buffering=0)
            assert "QMP" in json.loads(self.stream.readline())
            self.qmp("qmp_capabilities")
            return self
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def qmp(self, command, **arguments):
        self.stream.write(json.dumps({"execute": command, "arguments": arguments}).encode() + b"\n")
        while True:
            response = json.loads(self.stream.readline())
            if "event" not in response:
                assert "return" in response, response
                return response["return"]

    def api(self, path="diagnostics", data=None):
        if self.process.poll() is not None or "Kernel panic" in self.text():
            raise RuntimeError(f"guest failed to boot; see {self.serial}")
        body = json.dumps(data).encode() if data is not None else None
        request = Request(f"http://127.0.0.1:{self.args.port}/api/{path}", data=body,
                          headers={"Content-Type": "application/json"})
        with urlopen(request, timeout=3) as response:
            return json.load(response)

    def ready(self):
        result = wait("guest API", self.api, lambda data: data.get("status") == "ok")
        self.save("diagnostics", result)
        return result

    def save(self, suffix, data):
        (self.args.output / f"{self.name}-{suffix}.json").write_text(json.dumps(data, indent=2) + "\n")

    def text(self):
        return self.serial.read_text(errors="replace")

    def __exit__(self, *_):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        if hasattr(self, "stream"):
            self.stream.close()
            self.socket.close()
        self.log.close()
        self.qmp_path.unlink(missing_ok=True)


def main(args):
    assert (args.images / "rootfs.ext4").is_file(), "Run env -u BASH_ENV scripts/build-qemu.sh first"
    assert not args.output.exists(), "Choose a fresh output directory"
    args.output.mkdir(parents=True)
    run("sudo", "-n", "true")
    compiler = next((args.images.parent / "host/bin").glob("aarch64*-gcc"))
    source = args.output / "mount-fixtures.c"
    source.write_text(WRAPPER)
    wrapper = args.output / "resilience-init"
    run(compiler, "-static", "-O2", source, "-o", wrapper)
    base = args.output / "base.ext4"
    shutil.copyfile(args.images / "rootfs.ext4", base)
    with base.open("ab") as stream:
        stream.truncate(512 * 1024 * 1024)
    run(args.images.parent / "host/sbin/resize2fs", base)
    with mounted(base, args.output / "root-mount") as root:
        run("sudo", "-n", "cp", wrapper, root / "sbin/resilience-init")
        run("sudo", "-n", "mkdir", "-p", root / "var/lib/musicos/update")
        original = args.output / "original-init"
        shutil.copyfile(root / "sbin/init", original)
    expected = digest(original)
    results = {}

    def disk(name):
        target = args.output / f"{name}.ext4"
        shutil.copyfile(base, target)
        return target

    def fixture(name, content=None, fs="vfat"):
        target = args.output / f"{name}.{fs}"
        with target.open("wb") as stream:
            stream.truncate((128 if fs == "vfat" else 8) * 1024 * 1024)
        run("mkfs.vfat" if fs == "vfat" else "mkfs.ext4", target)
        with mounted(target, args.output / f"{name}-mount", fs) as mount:
            if fs == "ext4":
                run("sudo", "-n", "chmod", "777", mount)
            if content is not None:
                (mount / "musicos-init").write_bytes(content)
        return target

    def record(name, function):
        if args.scenario and name not in args.scenario:
            return
        try:
            detail = function()
            results[name] = {"status": "PASS", "detail": detail}
        except Exception as error:
            results[name] = {"status": "FAIL", "detail": str(error)}
        print(name, json.dumps(results[name]), flush=True)
        (args.output / "results.json").write_text(json.dumps(results, indent=2) + "\n")

    def malformed(name, payload, reason):
        rootdisk, usb = disk(name), fixture(name, payload)
        with mounted(rootdisk, args.output / f"{name}-seed") as root:
            pending = args.output / "pending-invalid"
            pending.write_bytes(b"existing pending update")
            run("sudo", "-n", "cp", pending, root / "var/lib/musicos/update/musicos-init.next")
        with Guest(args, name, rootdisk, usb) as guest:
            guest.ready()
            guest.api("sync-usb", {})
            wait("validation diagnostic", guest.text, lambda text: reason in text)
        with mounted(rootdisk, args.output / f"{name}-inspect") as root:
            assert digest(root / "sbin/init") == expected, "target changed"
            stage = root / "var/lib/musicos/update"
            assert (stage / "musicos-init.next").read_bytes() == pending.read_bytes(), "pending changed"
            diagnostic = json.loads((stage / "last-update-result").read_text())
            assert diagnostic["operation"] == "stage" and diagnostic["outcome"] == "failure", diagnostic
            assert reason in diagnostic["reason"], diagnostic
            (args.output / f"{name}-result.json").write_text(json.dumps(diagnostic, indent=2))
        return {"target_sha256": expected, "diagnostic": diagnostic}

    wrong = bytearray(original.read_bytes()[:4096])
    struct.pack_into("<H", wrong, 18, 62)
    record("wrong-arch", lambda: malformed("wrong-arch", wrong, "architecture mismatch"))
    truncated = bytearray(original.read_bytes()[:64])
    record("truncated-elf", lambda: malformed("truncated-elf", truncated, "truncated ELF program header table"))
    squash = bytearray(96)
    squash[:4] = b"hsqs"
    struct.pack_into("<I", squash, 12, 131072)
    struct.pack_into("<H", squash, 28, 4)
    struct.pack_into("<Q", squash, 40, 4096)
    record("truncated-squashfs", lambda: malformed("truncated-squashfs", squash, "SquashFS bytes_used"))

    def interrupted():
        rootdisk = disk("interrupted")
        padded = args.output / "padded-init"
        shutil.copyfile(original, padded)
        with padded.open("ab") as stream:
            stream.truncate(max(padded.stat().st_size, 64 * 1024 * 1024))
        with mounted(rootdisk, args.output / "interrupted-seed") as root:
            stage = root / "var/lib/musicos/update"
            run("sudo", "-n", "cp", padded, stage / "musicos-init.next")
            run("sudo", "-n", "cp", source, stage / "musicos-init.tmp")
            run("sudo", "-n", "cp", source, root / "sbin/.init.installing")
        with Guest(args, "interrupted-copy", rootdisk, throttle=True) as guest:
            wait("in-progress install", guest.text, lambda text: "RESILIENCE installing exists" in text)
            wait("seeded leftover cleanup", guest.text, lambda text:
                 "removed interrupted update leftover /sbin/.init.installing" in text and
                 "removed interrupted update leftover /var/lib/musicos/update/musicos-init.tmp" in text)
            time.sleep(6)
        with Guest(args, "interrupted-reboot", rootdisk) as guest:
            guest.ready()
            wait("install diagnostic", guest.text, lambda text: '"outcome":"success"' in text)
        with Guest(args, "installed-init-reboot", rootdisk) as guest:
            guest.ready()
        with mounted(rootdisk, args.output / "interrupted-inspect") as root:
            assert digest(root / "sbin/init") == digest(padded), "new init not installed"
            assert digest(root / "var/lib/musicos/update/musicos-init.previous") == expected, "original backup changed"
            assert not (root / "sbin/.init.installing").exists()
            assert not (root / "var/lib/musicos/update/musicos-init.tmp").exists()
        return "Seeded .tmp/.installing cleaned at boot; QEMU killed during copy; pending update recovered and new executable rebooted"

    record("interrupted-install", interrupted)

    def removed():
        rootdisk, usb = disk("removed"), fixture("media")
        with mounted(usb, args.output / "media-mount", "vfat") as mount:
            mount = mount / "tracks"
            mount.mkdir()
            for number in range(2000):
                with wave.open(str(mount / f"tone-{number:04}.wav"), "wb") as tone:
                    tone.setparams((1, 2, 8000, 0, "NONE", "not compressed"))
                    tone.writeframes(struct.pack("<h", number + 1) * 8000)
        with Guest(args, "removed", rootdisk, usb) as guest:
            guest.ready()
            source = guest.api("library/sources", {"kind": "reference", "path": "/mnt/usb/sda", "display_name": "QEMU USB"})["source"]
            def tracks():
                data = guest.api("v2/tracks?limit=500")
                for offset in range(500, data["total"], 500):
                    data["items"].extend(guest.api(f"v2/tracks?limit=500&offset={offset}")["items"])
                return data
            initial = wait("initial library", tracks, lambda data: data.get("total") == 2000, timeout=240)
            guest.save("before", initial)
            wait("initial scan complete", guest.api, lambda data: data.get("scan", {}).get("state") != "running")
            # Trigger a full scan and establish reachability before removing the device.
            guest.api("library/rescan", {"mode": "full"})
            active = wait("active rescan", guest.api, lambda data:
                          data.get("scan", {}).get("state") == "running" and
                          data["scan"].get("discovered", 0) > 0 and
                          data["scan"].get("processed", 0) > 0)
            guest.save("active-scan-before-unplug", active)
            guest.qmp("device_del", id="removable")
            wait("USB disconnect", guest.text, lambda text: "USB disconnect" in text)
            wait("scan complete", guest.api, lambda data: data.get("scan", {}).get("state") != "running")
            remaining, sources = tracks(), guest.api("library/sources")
            guest.save("after", remaining)
            guest.save("sources", sources)
            assert len(remaining["items"]) == len(initial["items"]), "library rows lost"
            current = next(item for item in sources["items"] if item["id"] == source["id"])
            assert current["available"] is False, (
                f"source remains available after physical device_del; "
                f"rows before={len(initial['items'])}, after={len(remaining['items'])}; "
                f"active scan before unplug={active['scan']}")
        return "active reference scan unplugged; rows retained and source unavailable"

    record("removed-mid-scan", removed)

    def readonly():
        rootdisk, fault = disk("readonly"), fixture("readonly-data", fs="ext4")
        with Guest(args, "readonly", rootdisk, fault, "ext4", {
                "RESILIENCE_RO": "1", "MUSICOS_MEDIA_DB": "/mnt/fault/library.sqlite3",
                "MUSICOS_UPDATE_STAGING": "/mnt/fault/update"}) as guest:
            guest.ready()
            wait("persistence failure log", guest.text, lambda text: "using a temporary in-memory database" in text)
            assert guest.api()["status"] == "ok", "app stopped after storage failure"
        return "read-only tiny ext4 rejected persistent DB; PID 1 stayed alive using in-memory DB"

    record("readonly-db", readonly)
    return 0 if all(item["status"] == "PASS" for item in results.values()) else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--images", type=Path, default=ROOT / "out/qemu/images")
    parser.add_argument("--kernel", type=Path, help="Optional scratch kernel (e.g. with VFAT NLS enabled)")
    parser.add_argument("--scenario", action="append", choices=["wrong-arch", "truncated-elf",
                        "truncated-squashfs", "interrupted-install", "removed-mid-scan", "readonly-db"],
                        help="Run only selected scenarios (repeatable)")
    parser.add_argument("--port", type=int, default=18819)
    options = parser.parse_args()
    options.output = options.output.resolve()
    raise SystemExit(main(options))
