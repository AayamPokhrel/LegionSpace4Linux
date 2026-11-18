#!/usr/bin/env python3
"""
Legion Space Linux GUI Backend Server (v1.0.0)
High-performance REST API and static server bridging web frontend to Lenovo Legion hardware.
"""

import http.server
import json
import os
import re
import socketserver
import subprocess
import sys
import threading
import time
from urllib.parse import parse_qs, urlparse

PORT = 9090
STATIC_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "static")

# Cache for CPU usage calculation
_prev_cpu_times = None
_prev_cpu_time_stamp = 0.0
_prev_cpu_percent = 0.0


def get_cpu_utilization():
    global _prev_cpu_times, _prev_cpu_time_stamp, _prev_cpu_percent
    now = time.time()
    try:
        with open("/proc/stat", "r") as f:
            line = f.readline()
        fields = [float(x) for x in line.strip().split()[1:]]
        idle_time = fields[3] + (fields[4] if len(fields) > 4 else 0.0)
        total_time = sum(fields)

        if _prev_cpu_times is not None:
            prev_idle, prev_total = _prev_cpu_times
            idle_delta = idle_time - prev_idle
            total_delta = total_time - prev_total
            if total_delta > 0:
                _prev_cpu_percent = max(0.0, min(100.0, 100.0 * (1.0 - (idle_delta / total_delta))))

        _prev_cpu_times = (idle_time, total_time)
        _prev_cpu_time_stamp = now
        return round(_prev_cpu_percent, 1)
    except Exception:
        return 0.0


def get_cpu_frequency_ghz():
    try:
        with open("/proc/cpuinfo", "r") as f:
            lines = f.readlines()
        freqs = []
        for line in lines:
            if "cpu MHz" in line:
                freqs.append(float(line.split(":")[1].strip()))
        if freqs:
            avg_mhz = sum(freqs) / len(freqs)
            return round(avg_mhz / 1000.0, 2)
    except Exception:
        pass
    return 0.0


def get_cpu_temperature_c():
    for hwmon in ["/sys/class/hwmon/hwmon4/temp1_input", "/sys/class/hwmon/hwmon5/temp1_input", "/sys/class/thermal/thermal_zone0/temp"]:
        if os.path.exists(hwmon):
            try:
                with open(hwmon, "r") as f:
                    val = int(f.read().strip())
                    if val > 1000:
                        val = val // 1000
                    if 20 <= val <= 115:
                        return val
            except Exception:
                continue
    return 45


def get_ram_metrics():
    try:
        with open("/proc/meminfo", "r") as f:
            meminfo = {}
            for line in f:
                parts = line.split(":")
                if len(parts) == 2:
                    meminfo[parts[0].strip()] = parts[1].strip()

        total_kb = float(meminfo.get("MemTotal", "0 kB").split()[0])
        avail_kb = float(meminfo.get("MemAvailable", "0 kB").split()[0])
        used_kb = total_kb - avail_kb

        total_gb = round(total_kb / (1024.0 * 1024.0), 1)
        used_gb = round(used_kb / (1024.0 * 1024.0), 1)
        percent = round((used_kb / total_kb) * 100.0, 1) if total_kb > 0 else 0.0
        return {"total_gb": total_gb, "used_gb": used_gb, "percent": percent}
    except Exception:
        return {"total_gb": 32.0, "used_gb": 8.0, "percent": 25.0}


def get_gpu_metrics():
    try:
        res = subprocess.run(
            ["nvidia-smi", "--query-gpu=utilization.gpu,temperature.gpu,clocks.current.graphics,power.draw", "--format=csv,noheader,nounits"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=1.5,
        )
        if res.returncode == 0 and res.stdout.strip():
            parts = [p.strip() for p in res.stdout.strip().split(",")]
            if len(parts) >= 4:
                return {
                    "available": True,
                    "utilization_percent": int(float(parts[0])),
                    "temperature_c": int(float(parts[1])),
                    "clock_mhz": int(float(parts[2])),
                    "clock_ghz": round(float(parts[2]) / 1000.0, 2),
                    "power_w": round(float(parts[3]), 1),
                }
    except Exception:
        pass
    return {
        "available": False,
        "utilization_percent": 0,
        "temperature_c": 0,
        "clock_mhz": 0,
        "clock_ghz": 0.0,
        "power_w": 0.0,
    }


def get_legionctl_status():
    legionctl_bin = os.path.expanduser("~/.local/bin/legionctl")
    if not os.path.exists(legionctl_bin):
        legionctl_bin = "/usr/local/bin/legionctl"
    if not os.path.exists(legionctl_bin):
        legionctl_bin = "legionctl"

    try:
        res = subprocess.run([legionctl_bin, "status", "--json"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=2.0)
        if res.returncode == 0:
            return json.loads(res.stdout)
    except Exception as e:
        sys.stderr.write(f"legionctl status error: {e}\n")
    return {}


def run_legionctl_cmd(args):
    legionctl_bin = os.path.expanduser("~/.local/bin/legionctl")
    if not os.path.exists(legionctl_bin):
        legionctl_bin = "/usr/local/bin/legionctl"
    if not os.path.exists(legionctl_bin):
        legionctl_bin = "legionctl"

    cmd = [legionctl_bin] + args + ["--json"]
    try:
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=3.0)
        output = res.stdout.strip()
        try:
            return json.loads(output)
        except Exception:
            return {"success": res.returncode == 0, "output": output, "error": res.stderr.strip()}
    except Exception as e:
        return {"success": False, "error": str(e)}


class LegionRequestHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=STATIC_DIR, **kwargs)

    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path == "/api/status":
            self.handle_api_status()
        elif parsed.path == "/api/ping":
            self.send_json({"status": "pong", "time": time.time()})
        else:
            super().do_GET()

    def do_POST(self):
        parsed = urlparse(self.path)
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode("utf-8") if length > 0 else "{}"
        try:
            data = json.loads(body)
        except Exception:
            data = {}

        if parsed.path == "/api/profile":
            profile = data.get("profile", "balanced")
            res = run_legionctl_cmd(["profile", "set", profile])
            self.send_json(res)
        elif parsed.path == "/api/fan":
            mode = data.get("mode", "auto")
            if mode == "auto":
                res = run_legionctl_cmd(["fan", "auto"])
            elif mode == "full":
                res = run_legionctl_cmd(["fan", "full"])
            elif mode == "set":
                rpm = str(data.get("rpm", 3500))
                res = run_legionctl_cmd(["fan", "set", "--both", rpm])
            else:
                res = {"success": False, "error": f"Unknown fan mode: {mode}"}
            self.send_json(res)
        elif parsed.path == "/api/battery/conservation":
            state = "on" if data.get("enabled", True) else "off"
            res = run_legionctl_cmd(["battery", "conservation", state])
            self.send_json(res)
        elif parsed.path == "/api/battery/usb-charge":
            state = "on" if data.get("enabled", True) else "off"
            res = run_legionctl_cmd(["battery", "usb-charge", state])
            self.send_json(res)
        elif parsed.path == "/api/lighting/preset":
            preset = data.get("preset", "legion-blue")
            res = run_legionctl_cmd(["lighting", "preset", preset])
            self.send_json(res)
        elif parsed.path == "/api/lighting/static":
            zone = str(data.get("zone", "all"))
            color = str(data.get("color", "#00d3fe"))
            brightness = str(data.get("brightness", 3))
            res = run_legionctl_cmd(["lighting", "static", "--zone", zone, "--color", color, "--brightness", brightness])
            self.send_json(res)
        elif parsed.path == "/api/lighting/effect":
            mode = str(data.get("effect", "wave"))
            speed = str(data.get("speed", 2))
            brightness = str(data.get("brightness", 3))
            color = str(data.get("color", "cyan"))
            res = run_legionctl_cmd(["lighting", "effect", "--mode", mode, "--speed", speed, "--brightness", brightness, "--color", color])
            self.send_json(res)
        elif parsed.path == "/api/lighting/fn-lock":
            state = "on" if data.get("enabled", True) else "off"
            res = run_legionctl_cmd(["lighting", "fn-lock", state])
            self.send_json(res)
        elif parsed.path == "/api/custom/set":
            attr = data.get("attribute", "")
            val = str(data.get("value", ""))
            # Custom set supports: --pl1, --pl2, --pl3, --cpu-temp, --gpu-temp, --nv-offset
            mapping = {
                "ppt_pl1_spl": "--pl1",
                "ppt_pl2_sppt": "--pl2",
                "ppt_pl3_fppt": "--pl3",
                "cpu_temp": "--cpu-temp",
                "gpu_temp": "--gpu-temp",
                "gpu_nv_ac_offset": "--nv-offset",
            }
            if attr in mapping:
                res = run_legionctl_cmd(["custom", "set", mapping[attr], val])
            else:
                res = {"success": False, "error": f"Unsupported attribute: {attr}"}
            self.send_json(res)
        else:
            self.send_error(404, "Endpoint not found")

    def handle_api_status(self):
        status = get_legionctl_status()
        cpu_pct = get_cpu_utilization()
        cpu_ghz = get_cpu_frequency_ghz()
        cpu_temp = get_cpu_temperature_c()
        ram = get_ram_metrics()
        gpu = get_gpu_metrics()

        payload = {
            "dmi": status.get("dmi", {
                "product_family": "Legion Pro 5 16ADR10",
                "product_name": "83LT",
                "bios_version": "RLCN33WW",
            }),
            "telemetry": {
                "cpu": {
                    "usage_percent": cpu_pct,
                    "clock_ghz": cpu_ghz,
                    "temperature_c": cpu_temp,
                    "fan_rpm": status.get("fans", {}).get("fans", [{}])[0].get("current_rpm", 0) if status.get("fans", {}).get("fans") else 0,
                },
                "ram": ram,
                "gpu": {
                    "available": gpu["available"],
                    "usage_percent": gpu["utilization_percent"],
                    "clock_ghz": gpu["clock_ghz"],
                    "clock_mhz": gpu["clock_mhz"],
                    "temperature_c": gpu["temperature_c"],
                    "power_w": gpu["power_w"],
                    "fan_rpm": status.get("fans", {}).get("fans", [{}, {}])[1].get("current_rpm", 0) if len(status.get("fans", {}).get("fans", [])) > 1 else 0,
                },
            },
            "thermal": status.get("thermal", {}),
            "fans": status.get("fans", {}),
            "battery": status.get("battery", {}),
            "lighting": status.get("lighting", {}),
            "custom": status.get("custom", {}),
        }
        self.send_json(payload)

    def send_json(self, data, status_code=200):
        body = json.dumps(data).encode("utf-8")
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        # Silence routine static asset logs
        if "api/status" not in (args[0] if args else ""):
            super().log_message(format, *args)


class ThreadedTCPServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


def run_server(port=PORT):
    server = ThreadedTCPServer(("127.0.0.1", port), LegionRequestHandler)
    print(f"Legion Space Backend running at http://127.0.0.1:{port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping Legion Space Backend...")
        server.shutdown()


def daemonize():
    try:
        pid = os.fork()
        if pid > 0:
            sys.exit(0)
    except OSError as e:
        sys.stderr.write(f"fork #1 failed: {e}\n")
        sys.exit(1)

    os.setsid()
    os.umask(0)

    try:
        pid = os.fork()
        if pid > 0:
            sys.exit(0)
    except OSError as e:
        sys.stderr.write(f"fork #2 failed: {e}\n")
        sys.exit(1)

    sys.stdout.flush()
    sys.stderr.flush()
    config_dir = os.path.expanduser("~/.config/legionspace")
    os.makedirs(config_dir, exist_ok=True)
    log_file = open(os.path.join(config_dir, "server.log"), "a+")
    os.dup2(log_file.fileno(), sys.stdout.fileno())
    os.dup2(log_file.fileno(), sys.stderr.fileno())


if __name__ == "__main__":
    if "--daemon" in sys.argv:
        daemonize()
    run_server()
