#!/usr/bin/env python3
import json
import os
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs


def _display(value: float, unit: str, format_hint: str) -> str:
    if format_hint == "decibel":
        return f"{(value * 24.0 - 12.0):+.1f} dB"
    if format_hint == "frequency":
        return f"{(20.0 * ((20000.0 / 20.0) ** value)):.0f} Hz"
    if unit == "%":
        return f"{value * 100.0:.0f}%"
    return f"{value:.3f}"


def plugin_template(name: str):
    n = name.lower()
    if "reagate" in n or "gate" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Gate Enable", "value": 1.0, "unit": "", "format_hint": "raw"},
                {"name": "Threshold", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Attack", "value": 0.2, "unit": "", "format_hint": "raw"},
                {"name": "Release", "value": 0.4, "unit": "", "format_hint": "raw"},
            ],
        }
    if "compressor" in n or "comp" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Compressor Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Threshold", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Ratio", "value": 0.25, "unit": "", "format_hint": "raw"},
                {"name": "Attack", "value": 0.2, "unit": "", "format_hint": "raw"},
                {"name": "Release", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Makeup", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Mix", "value": 1.0, "unit": "", "format_hint": "raw"},
            ],
        }
    if "overdrive" in n or "tubescreamer" in n or "screamer" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Overdrive Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Drive", "value": 0.3, "unit": "", "format_hint": "raw"},
                {"name": "Tone", "value": 0.55, "unit": "", "format_hint": "raw"},
                {"name": "Level", "value": 0.6, "unit": "", "format_hint": "raw"},
            ],
        }
    if "distortion" in n or "fuzz" in n or "hm-2" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Distortion Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Drive", "value": 0.6, "unit": "", "format_hint": "raw"},
                {"name": "Tone", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Level", "value": 0.6, "unit": "", "format_hint": "raw"},
                {"name": "Low", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "High", "value": 0.5, "unit": "", "format_hint": "raw"},
            ],
        }
    if "chorus" in n or "mod" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Chorus Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Rate", "value": 0.3, "unit": "", "format_hint": "raw"},
                {"name": "Depth", "value": 0.4, "unit": "", "format_hint": "raw"},
                {"name": "Mix", "value": 0.25, "unit": "", "format_hint": "raw"},
            ],
        }
    if "readelay" in n or "delay" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Delay Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Delay Time", "value": 0.3, "unit": "", "format_hint": "raw"},
                {"name": "Delay Feedback", "value": 0.2, "unit": "", "format_hint": "raw"},
                {"name": "Delay Mix", "value": 0.1, "unit": "", "format_hint": "raw"},
            ],
        }
    if "reaverbate" in n or "reverbate" in n or "reverb" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Reverb Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Pre-delay", "value": 0.15, "unit": "", "format_hint": "raw"},
                {"name": "Decay", "value": 0.35, "unit": "", "format_hint": "raw"},
                {"name": "High Cut", "value": 0.8, "unit": "", "format_hint": "raw"},
                {"name": "Low Cut", "value": 0.1, "unit": "", "format_hint": "raw"},
                {"name": "Room Size", "value": 0.25, "unit": "", "format_hint": "raw"},
                {"name": "Mix", "value": 0.1, "unit": "", "format_hint": "raw"},
            ],
        }
    if "reaeq" in n or "eq" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "EQ Bypass", "value": 0.0, "unit": "", "format_hint": "raw"},
                {"name": "Band 1 Freq", "value": 0.4, "unit": "Hz", "format_hint": "frequency"},
                {"name": "Band 1 Gain", "value": 0.5, "unit": "dB", "format_hint": "decibel"},
                {"name": "Band 1 Q", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Band 2 Freq", "value": 0.55, "unit": "Hz", "format_hint": "frequency"},
                {"name": "Band 2 Gain", "value": 0.5, "unit": "dB", "format_hint": "decibel"},
                {"name": "Band 2 Q", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Band 3 Freq", "value": 0.65, "unit": "Hz", "format_hint": "frequency"},
                {"name": "Band 3 Gain", "value": 0.5, "unit": "dB", "format_hint": "decibel"},
                {"name": "Band 3 Q", "value": 0.5, "unit": "", "format_hint": "raw"},
                {"name": "Band 4 Freq", "value": 0.75, "unit": "Hz", "format_hint": "frequency"},
                {"name": "Band 4 Gain", "value": 0.5, "unit": "dB", "format_hint": "decibel"},
                {"name": "Band 4 Q", "value": 0.5, "unit": "", "format_hint": "raw"},
            ],
        }
    if "neural" in n or "archetype" in n or "amp" in n:
        return {
            "name": name,
            "enabled": True,
            "params": [
                {"name": "Gain", "value": 0.5, "unit": "%", "format_hint": "percentage"},
                {"name": "Input", "value": 0.5, "unit": "%", "format_hint": "percentage"},
                {"name": "Drive", "value": 0.5, "unit": "%", "format_hint": "percentage"},
                {"name": "Bass", "value": 0.5, "unit": "%", "format_hint": "percentage"},
                {"name": "Mid", "value": 0.5, "unit": "%", "format_hint": "percentage"},
                {"name": "Treble", "value": 0.5, "unit": "%", "format_hint": "percentage"},
                {"name": "Presence", "value": 0.5, "unit": "%", "format_hint": "percentage"},
            ],
        }
    return {"name": name, "enabled": True, "params": []}


def scenario_state(scenario: str):
    scenario = (scenario or "baseline").lower()

    if scenario == "confusing_delay_section":
        delay = plugin_template("Delay FX")
        delay["params"][0]["value"] = 1.0  # Delay Bypass ON
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype Gojira"),
                        delay,
                    ],
                }
            ]
        }

    if scenario == "disabled_gate":
        gate = plugin_template("ReaGate (Cockos)")
        gate["enabled"] = False
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        gate,
                    ],
                }
            ]
        }

    if scenario == "missing_reverb":
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        plugin_template("ReaGate (Cockos)"),
                    ],
                }
            ]
        }

    if scenario == "bypassed_reverb":
        reverb = plugin_template("ReaVerbate (Cockos)")
        reverb["params"][0]["value"] = 1.0  # Reverb Bypass ON
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        reverb,
                    ],
                }
            ]
        }

    if scenario == "gate_enable_off":
        gate = plugin_template("ReaGate (Cockos)")
        gate["params"][0]["value"] = 0.0  # Gate Enable OFF
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        gate,
                    ],
                }
            ]
        }

    if scenario == "bypassed_eq":
        eq = plugin_template("ReaEQ (Cockos)")
        eq["params"][0]["value"] = 1.0  # EQ Bypass ON
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        eq,
                    ],
                }
            ]
        }

    if scenario == "dual_delay_prefer_readelay":
        generic = plugin_template("Delay FX")
        readelay = plugin_template("ReaDelay (Cockos)")
        readelay["params"][0]["value"] = 1.0  # bypass on
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        generic,
                        readelay,
                    ],
                }
            ]
        }

    if scenario == "shoegaze_wall":
        chorus = plugin_template("Chorus Mod")
        chorus["params"][0]["value"] = 1.0  # bypass on
        delay = plugin_template("ReaDelay (Cockos)")
        delay["params"][0]["value"] = 1.0  # bypass on
        reverb = plugin_template("ReaVerbate (Cockos)")
        reverb["params"][0]["value"] = 1.0  # bypass on
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        chorus,
                        delay,
                        reverb,
                    ],
                }
            ]
        }

    if scenario == "chainsaw_distortion_bypassed":
        dist = plugin_template("HM-2 Distortion")
        dist["params"][0]["value"] = 1.0  # bypass on
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        dist,
                        plugin_template("ReaEQ (Cockos)"),
                    ],
                }
            ]
        }

    if scenario == "funk_compressor_disabled":
        comp = plugin_template("Compressor")
        comp["enabled"] = False
        comp["params"][0]["value"] = 1.0  # bypass on
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        comp,
                    ],
                }
            ]
        }

    if scenario == "overdrive_bypassed":
        od = plugin_template("TubeScreamer Overdrive")
        od["params"][0]["value"] = 1.0  # bypass on
        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        plugin_template("VST3: Neural DSP Archetype"),
                        od,
                    ],
                }
            ]
        }

    if scenario == "kitchen_sink":
        # Multiple confusing modules:
        # - Amp disabled
        # - Delay bypassed and clamps time to <= 0.9 (simulated)
        amp = plugin_template("VST3: Neural DSP Archetype")
        amp["enabled"] = False

        delay = plugin_template("ReaDelay (Cockos)")
        delay["params"][0]["value"] = 1.0  # bypass on

        return {
            "tracks": [
                {
                    "name": "Guitar",
                    "fx": [
                        amp,
                        delay,
                    ],
                }
            ]
        }

    return {
        "tracks": [
            {
                "name": "Guitar",
                "fx": [
                    plugin_template("VST3: Neural DSP Archetype"),
                    plugin_template("ReaGate (Cockos)"),
                    plugin_template("ReaDelay (Cockos)"),
                ],
            }
        ]
    }


class State:
    def __init__(self):
        self.lock = threading.Lock()
        self.data = scenario_state(os.environ.get("MOCK_SCENARIO", "baseline"))

    def reset(self, scenario: str):
        with self.lock:
            self.data = scenario_state(scenario)


STATE = State()


class Handler(BaseHTTPRequestHandler):
    def _send(self, status: int, payload: dict):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json(self):
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length > 0 else b"{}"
        return json.loads(raw.decode("utf-8"))

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        qs = parse_qs(parsed.query)

        if path == "/ping":
            return self._send(200, {"status": "ok", "service": "Mock REAPER Extension"})

        if path == "/__reset":
            scenario = qs.get("scenario", ["baseline"])[0]
            STATE.reset(scenario)
            return self._send(200, {"ok": True, "scenario": scenario})

        if path == "/tracks":
            with STATE.lock:
                tracks = []
                for ti, t in enumerate(STATE.data["tracks"]):
                    fx_list = []
                    for fi, fx in enumerate(t["fx"]):
                        fx_list.append(
                            {"index": fi, "name": fx["name"], "enabled": bool(fx.get("enabled", True))}
                        )
                    tracks.append(
                        {"index": ti, "name": t["name"], "fx_count": len(t["fx"]), "fx_list": fx_list}
                    )
            return self._send(200, {"track_count": len(tracks), "tracks": tracks})

        if path == "/fx/params":
            track = int(qs.get("track", ["0"])[0])
            fx = int(qs.get("fx", ["0"])[0])
            with STATE.lock:
                try:
                    plugin = STATE.data["tracks"][track]["fx"][fx]
                except Exception:
                    return self._send(404, {"error": "Not found"})
                params = []
                for idx, p in enumerate(plugin["params"]):
                    params.append(
                        {
                            "index": idx,
                            "name": p["name"],
                            "value": float(p["value"]),
                            "display": _display(float(p["value"]), p.get("unit", ""), p.get("format_hint", "raw")),
                            "unit": p.get("unit", ""),
                            "format_hint": p.get("format_hint", "raw"),
                        }
                    )
            return self._send(200, {"track": track, "fx": fx, "params": params})

        if path == "/fx/param_index":
            track = int(qs.get("track", ["0"])[0])
            fx = int(qs.get("fx", ["0"])[0])
            param_index = int(qs.get("param_index", ["-1"])[0])
            with STATE.lock:
                try:
                    plugin = STATE.data["tracks"][track]["fx"][fx]
                    p = plugin["params"][param_index]
                except Exception:
                    return self._send(404, {"error": "Not found"})
            return self._send(
                200,
                {
                    "track": track,
                    "fx": fx,
                    "param_index": param_index,
                    "param_name": p["name"],
                    "value": float(p["value"]),
                },
            )

        return self._send(404, {"error": "Unknown endpoint", "path": path})

    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path
        body = self._read_json()

        if path == "/fx/toggle":
            track = int(body.get("track", 0))
            fx = int(body.get("fx", 0))
            enabled = bool(body.get("enabled", True))
            with STATE.lock:
                try:
                    plugin = STATE.data["tracks"][track]["fx"][fx]
                except Exception:
                    return self._send(404, {"error": "Not found"})
                plugin["enabled"] = enabled
            return self._send(200, {"success": True, "track": track, "fx": fx, "enabled": enabled})

        if path == "/fx/add":
            track = int(body.get("track", 0))
            plugin_name = body.get("plugin", "")
            if not plugin_name:
                return self._send(400, {"error": "plugin required"})
            with STATE.lock:
                try:
                    fx_list = STATE.data["tracks"][track]["fx"]
                except Exception:
                    return self._send(404, {"error": "Track not found"})
                fx_list.append(plugin_template(plugin_name))
                fx_index = len(fx_list) - 1
            return self._send(
                200,
                {
                    "success": True,
                    "track": track,
                    "fx_index": fx_index,
                    "fx_name": plugin_name,
                },
            )

        if path == "/fx/param_index":
            track = int(body.get("track", 0))
            fx = int(body.get("fx", 0))
            param_index = int(body.get("param_index", -1))
            value = float(body.get("value", 0.0))
            with STATE.lock:
                try:
                    plugin = STATE.data["tracks"][track]["fx"][fx]
                    p = plugin["params"][param_index]
                except Exception:
                    return self._send(404, {"error": "Not found"})

                # Simulate plugin constraints for one scenario
                if "readelay" in plugin["name"].lower() and p["name"].lower() == "delay time":
                    if value > 0.9:
                        value = 0.9

                value = max(0.0, min(1.0, value))
                p["value"] = value

            return self._send(
                200,
                {
                    "success": True,
                    "track": track,
                    "fx": fx,
                    "param_index": param_index,
                    "param_name": p["name"],
                    "value": float(p["value"]),
                },
            )

        return self._send(404, {"error": "Unknown endpoint", "path": path})

    def log_message(self, *_args):
        # Silence logs for cleaner test output.
        return


def main():
    host = os.environ.get("MOCK_HOST", "127.0.0.1")
    port = int(os.environ.get("MOCK_PORT", "8888"))
    server = HTTPServer((host, port), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()
