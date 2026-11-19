#!/usr/bin/env python3
"""Smoke-test the aarch64 Buildroot image using QMP input and a recorded PCM sink."""

import argparse
import array
import hashlib
import json
import math
import shlex
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time
import wave
from contextlib import contextmanager
from pathlib import Path
from urllib.error import URLError
from urllib.request import Request, urlopen


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def wait_for(label, read, ready=bool, timeout=30):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            last = read()
            if ready(last):
                return last
        except (URLError, TimeoutError) as err:
            last = str(err)
        time.sleep(0.25)
    raise RuntimeError(f"{label} timed out (last result: {last!r})")


class Qmp:
    def __init__(self, stream, size):
        self.stream = stream
        self.size = size
        require("QMP" in self.receive(), "Missing QMP greeting")
        self.command("qmp_capabilities")

    def receive(self):
        data = self.stream.readline()
        require(data, "QEMU closed the QMP connection")
        return json.loads(data)

    def command(self, name, **arguments):
        self.stream.write(json.dumps({"execute": name, "arguments": arguments}).encode() + b"\n")
        while True:
            response = self.receive()
            if "event" not in response:
                require("return" in response, f"QMP {name}: {response}")
                return response["return"]

    def keys(self, *keys):
        for key in keys:
            self.command("send-key", keys=[{"type": "qcode", "data": key}], **{"hold-time": 80})
            time.sleep(0.4)

    def screenshot(self, path):
        self.command("screendump", filename=str(path), format="png")
        with path.open("rb") as png:
            header = png.read(24)
        require(len(header) == 24 and header.startswith(b"\x89PNG\r\n\x1a\n"),
                f"Invalid PNG framebuffer capture: {path.name}")
        size = struct.unpack(">II", header[16:24])
        require(size == self.size, f"{path.name}: expected display {self.size}, got {size}")


@contextmanager
def boot(args, disk, output, name):
    width, height = args.resolution.split("x")
    with tempfile.TemporaryDirectory(prefix="musicos-qmp-") as tmp:
        qmp_path = Path(tmp) / "socket"
        command = [
            args.qemu, "-M", "virt", "-cpu", "cortex-a53", "-smp", "4", "-m", "512M",
            "-kernel", str(args.images / "Image"),
            "-drive", f"file={disk},if=none,format=raw,id=hd0,cache=writethrough",
            "-device", "virtio-blk-pci,drive=hd0",
            "-device", f"virtio-gpu-pci,xres={width},yres={height}",
            "-device", "virtio-keyboard-pci", "-device", "virtio-tablet-pci",
            "-device", "virtio-sound-pci,audiodev=snd0",
            "-audiodev", f"wav,id=snd0,path={output / (name + '.wav')}",
            "-display", "none", "-monitor", "none", "-no-reboot",
            "-serial", f"file:{output / (name + '-serial.log')}",
            "-qmp", f"unix:{qmp_path},server=on,wait=off",
            "-netdev", f"user,id=net0,hostfwd=tcp:127.0.0.1:{args.port}-:8080",
            "-device", "virtio-net-pci,netdev=net0",
            "-append", f"console=ttyAMA0 root=/dev/vda rw rootwait init=/sbin/init "
                       f"MUSICOS_RESOLUTION={args.resolution} "
                       "ip=10.0.2.15::10.0.2.2:255.255.255.0::eth0:off",
        ]
        (output / f"{name}-command.txt").write_text(shlex.join(command) + "\n")
        with (output / f"{name}-qemu.log").open("wb") as log:
            process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
            try:
                def qmp_ready():
                    require(process.poll() is None, f"QEMU exited; see {name}-qemu.log")
                    return qmp_path.is_socket()

                wait_for("QMP socket", qmp_ready, timeout=10)
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
                    sock.settimeout(10)
                    sock.connect(str(qmp_path))
                    with sock.makefile("rwb", buffering=0) as stream:
                        qmp = Qmp(stream, (int(width), int(height)))
                        try:
                            yield qmp
                        finally:
                            if process.poll() is None:
                                qmp.command("quit")
                require(process.wait(10) == 0, f"QEMU failed; see {name}-qemu.log")
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()


def make_tone(path):
    rate = 48000
    second = array.array("h", (int(10000 * math.sin(2 * math.pi * 440 * i / rate))
                               for i in range(rate)))
    if sys.byteorder != "little":
        second.byteswap()
    with wave.open(str(path), "wb") as wav:
        wav.setparams((1, 2, rate, 0, "NONE", "not compressed"))
        wav.writeframes(second.tobytes() * 20)


def run(args):
    output = args.output
    require((args.images / "Image").is_file(), "Run scripts/build-qemu.sh first (missing Image)")
    source = args.images / "rootfs.ext4"
    require(source.is_file(), "Run scripts/build-qemu.sh first (missing rootfs.ext4)")
    output.mkdir(parents=True)
    fixture = output / "Baseline-Tone.wav"
    make_tone(fixture)

    def api(path, data=None, headers=None):
        req = Request(f"http://127.0.0.1:{args.port}/api/{path}", data=data, headers=headers or {})
        with urlopen(req, timeout=2) as response:
            return json.load(response)

    def save(name, data):
        (output / f"{name}.json").write_text(json.dumps(data, indent=2) + "\n")

    def ready(name, qmp):
        result = wait_for("guest companion API", lambda: api("diagnostics"),
                          lambda data: data["status"] == "ok", timeout=90)
        save(f"{name}-diagnostics", result)
        require(result["db_path"] == "/var/lib/musicos/media.sqlite3", "Unexpected media DB path")
        serial = (output / f"{name}-serial.log").read_text(errors="replace")
        require("VFS: Mounted root (ext4 filesystem)" in serial, "Guest did not mount the ext4 root")
        require("Run /sbin/init as init process" in serial, "Kernel did not launch /sbin/init")
        require("opened input device" in serial, "Guest did not open any evdev input")
        require(f"MusicOS starting at {args.resolution}" in serial
                and "no framebuffer device found" not in serial,
                "Guest did not start with a framebuffer at the requested logical resolution")
        require("virtio" in (result["audio"]["proc_asound_cards"] or "").lower(),
                "No virtio ALSA card in guest diagnostics")
        qmp.screenshot(output / f"{name}-screen.png")

    with tempfile.TemporaryDirectory(prefix="musicos-rootfs-") as tmp:
        disk = Path(tmp) / "rootfs.ext4"
        shutil.copyfile(source, disk)
        with boot(args, disk, output, "boot") as qmp:
            ready("boot", qmp)
            initial = api("settings")
            require(not initial["setup_complete"], "Use a fresh Buildroot image for first-run checks")
            qmp.keys("ret", "ret", "ret")
            wait_for("first-run setup via evdev", lambda: api("settings"), lambda data: data["setup_complete"])
            qmp.screenshot(output / "home.png")

            body = fixture.read_bytes()
            headers = {"x-relative-path": f"baseline/music/{fixture.name}",
                       "x-sha256": hashlib.sha256(body).hexdigest()}
            upload = api("upload", body, headers)
            save("boot-upload", upload)
            require(upload["status"] == "ok", f"Tone upload failed: {upload}")
            tracks = wait_for("indexed test track", lambda: api("tracks"), lambda data: len(data) == 1)
            track_id = tracks[0]["id"]
            require(tracks[0]["path"].endswith("/" + fixture.name) and tracks[0]["play_count"] == 0,
                    f"Unexpected track: {tracks[0]}")
            save("boot-tracks", tracks)
            qmp.keys("ret", "down", "ret")
            qmp.screenshot(output / "songs.png")
            qmp.keys("ret")
            playing = wait_for("ALSA playback from Songs", lambda: api("player"),
                               lambda data: (data.get("player") or {}).get("position", 0) > 1)
            player = playing["player"]
            save("playing", playing)
            require(player["track"]["id"] == track_id and player["is_playing"]
                    and player["backend"] == "alsa" and not player["last_error"],
                    f"Unexpected playback state: {player}")
            qmp.screenshot(output / "now-playing.png")
            qmp.keys("ret")
            wait_for("pause via evdev", lambda: api("player"),
                     lambda data: not data["player"]["is_playing"])
            qmp.keys("y", "up", "ret", "down", "ret", "ret")
            settings = wait_for("Settings volume via evdev", lambda: api("settings"),
                                lambda data: data["volume"] > initial["volume"] + 0.05)
            save("boot-settings", settings)
            qmp.screenshot(output / "settings.png")
            played_tracks = api("tracks")
            save("played-tracks", played_tracks)
            require(len(played_tracks) == 1 and played_tracks[0]["play_count"] == 1
                    and played_tracks[0]["file_size"] == len(body),
                    f"Uploaded file size or play count not recorded before shutdown: {played_tracks}")

        with wave.open(str(output / "boot.wav"), "rb") as audio:
            require(audio.getsampwidth() == 2, "Expected a 16-bit QEMU WAV recording")
            pcm = array.array("h", audio.readframes(audio.getnframes()))
            if sys.byteorder != "little":
                pcm.byteswap()
            peak = max(map(abs, pcm), default=0)
            require(peak > 100, "QEMU WAV capture is silent")
            save("audio", {"frames": audio.getnframes(), "rate": audio.getframerate(),
                           "channels": audio.getnchannels(), "peak": peak})

        with boot(args, disk, output, "restart") as qmp:
            ready("restart", qmp)
            restarted = api("settings")
            tracks = api("tracks")
            save("restart-settings", restarted)
            save("restart-tracks", tracks)
            require(restarted["setup_complete"] and restarted["volume"] == settings["volume"],
                    "Setup or volume setting was lost on restart")
            require(len(tracks) == 1 and tracks[0]["id"] == track_id
                    and tracks[0]["play_count"] == 1 and tracks[0]["file_size"] == len(body)
                    and tracks[0]["last_played"] == played_tracks[0]["last_played"] is not None,
                    f"Indexed track, file size or play count did not survive restart: {tracks}")
            retry = api("upload", body, {**headers, "x-trigger-scan": "0"})
            save("restart-upload", retry)
            require(retry["status"] == "skipped", "Uploaded bytes did not survive restart")
    print(f"PASS: PID 1, evdev setup and navigation, ALSA PCM, and persistent settings/DB: {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--images", type=lambda p: Path(p).resolve(),
                        default=Path(__file__).resolve().parents[1] / "out/qemu/images")
    parser.add_argument("--qemu", default="qemu-system-aarch64")
    parser.add_argument("--resolution", choices=["640x480", "1280x960"], default="640x480")
    parser.add_argument("--port", type=int, default=18081)
    parser.add_argument("output", type=lambda p: Path(p).resolve(), help="new results directory")
    try:
        run(parser.parse_args())
    except (OSError, RuntimeError, wave.Error) as error:
        parser.exit(1, f"QEMU smoke failed: {error}\n")
