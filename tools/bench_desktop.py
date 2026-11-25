#!/usr/bin/env python3
"""Measures the desktop apps on Hyprland: time to the first window, memory
(PSS, RSS, private), idle CPU and threads. Median of 3 runs each.

    tools/bench_desktop.py target/release/ledger-desktop [../ledger-mru-flutter/build/linux/x64/release/bundle/ledger_mru]

Needs Hyprland (`hyprctl`) to see when the window appears. Uses your real data
folder: run it on demo data, or set XDG_DATA_HOME to a throwaway folder.
"""
import json, os, signal, statistics, subprocess, sys, time

if len(sys.argv) < 2:
    sys.exit(__doc__)
APPS = {os.path.basename(p): os.path.abspath(p) for p in sys.argv[1:]}
HZ = os.sysconf("SC_CLK_TCK")

def window_of(pid):
    out = subprocess.run(["hyprctl", "clients", "-j"], capture_output=True, text=True).stdout
    for c in json.loads(out or "[]"):
        if c.get("pid") == pid:
            return c
    return None

def ticks(pid):
    f = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
    return int(f[11]) + int(f[12])

def smaps(pid):
    d = {}
    for line in open(f"/proc/{pid}/smaps_rollup"):
        k, v = line.split(":", 1)
        if k in ("Pss", "Rss", "Private_Dirty"):
            d[k] = int(v.split()[0]) / 1024
    return d

def threads(pid):
    return int([l for l in open(f"/proc/{pid}/status") if l.startswith("Threads")][0].split()[1])

res = {}
for name, exe in APPS.items():
    runs = []
    for i in range(3):
        t0 = time.perf_counter()
        p = subprocess.Popen([exe], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        win = None
        while time.perf_counter() - t0 < 15:
            win = window_of(p.pid)
            if win: break
            time.sleep(0.01)
        t_win = (time.perf_counter() - t0) * 1000
        time.sleep(3)                        # let it settle (first frames, fonts, data)
        mem = smaps(p.pid)
        a = ticks(p.pid); time.sleep(8); b = ticks(p.pid)
        cpu = (b - a) / HZ / 8 * 100
        runs.append(dict(window_ms=t_win, pss=mem["Pss"], rss=mem["Rss"], priv=mem["Private_Dirty"],
                         cpu=cpu, threads=threads(p.pid), size=win and win["size"]))
        p.send_signal(signal.SIGTERM); p.wait(timeout=10)
        time.sleep(1)
    res[name] = runs

for name, runs in res.items():
    med = lambda k: statistics.median(r[k] for r in runs)
    print(f"{name:14} window {med('window_ms'):6.0f} ms | PSS {med('pss'):6.1f} MiB | RSS {med('rss'):6.1f} | private {med('priv'):6.1f} | CPU idle {med('cpu'):4.2f} % | threads {med('threads'):.0f} | win {runs[0]['size']}")
    print("   runs:", [round(r['window_ms']) for r in runs], [round(r['pss'],1) for r in runs])
