import pefile
import re
import os
import glob

def extract_strings(filepath, min_len=4):
    with open(filepath, "rb") as f:
        data = f.read()
    # ASCII strings
    ascii_strings = [s.decode("latin-1") for s in re.findall(b"[\x20-\x7e]{" + str(min_len).encode() + b",}", data)]
    # UTF-16LE strings
    utf16_strings = []
    for s in re.findall(b"(?:[\x20-\x7e]\x00){" + str(min_len).encode() + b",}", data):
        try:
            utf16_strings.append(s.decode("utf-16le"))
        except:
            pass
    return ascii_strings + utf16_strings

def search_keywords(strings, keywords):
    results = {}
    for kw in keywords:
        results[kw] = [s for s in strings if re.search(r'\b' + re.escape(kw) + r'\b', s, re.IGNORECASE) or kw.lower() in s.lower()]
    return results

home_dir = os.path.expanduser("~")
dll_base = os.environ.get("LEGION_DLL_DIR", os.path.join(home_dir, "Legion_Space", "extracted"))

dlls = [
    os.path.join(dll_base, "se/plugins/SEThermalModePlugin/SEThermalModePlugin64.dll"),
    os.path.join(dll_base, "se/plugins/SEPowerBatteryPlugin/SEPowerBatteryPlugin64.dll"),
    os.path.join(dll_base, "se/plugins/SYSCore/SYSCore64.dll"),
    os.path.join(dll_base, "se/lgtplugins/SEBasicLighting/SEBasicLighting64.dll"),
    os.path.join(dll_base, "se/plugins/SELuminousNB/SELuminousNB64.dll"),
]

for d in dlls:
    print("=" * 60)
    print("ANALYZING:", os.path.basename(d))
    print("=" * 60)
    s = extract_strings(d)
    
    # Check WMI / ACPI / DeviceIoControl / HID / Registry / Methods
    kw_groups = [
        "wmi", "acpi", "root\\wmi", "root\\cimv2", "deviceiocontrol", "createfile", 
        "thermal", "fan", "battery", "conservation", "rapid", "quiet", "performance", "balance",
        "hid_send", "hid_get", "feature_report", "writefile", "custom_mode", "fan_table", "overclock",
        "ec", "embedded_controller", "wmicall", "method", "setthermal", "getthermal"
    ]
    matches = search_keywords(s, kw_groups)
    for k, v in matches.items():
        if v:
            unique_v = list(set(v))[:15]
            print(f"[{k}] ({len(v)} matches):")
            for item in unique_v:
                print(f"   {item}")

