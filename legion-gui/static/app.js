/* ==============================================================================
   Legion Space Client Logic & Real-Time Telemetry Engine (v1.0.0)
   ============================================================================== */

let currentZoneTarget = "all";
let fanSliderDebounceTimer = null;
let customSliderDebounceTimer = null;

// Initialize when DOM ready
document.addEventListener("DOMContentLoaded", () => {
  fetchStatus();
  setInterval(fetchStatus, 1000);
  setupProfileButtons();
});

// Periodic Status Poller
async function fetchStatus() {
  try {
    const res = await fetch("/api/status");
    if (!res.ok) return;
    const data = await res.json();
    renderStatus(data);
  } catch (err) {
    console.error("Telemetry sync error:", err);
  }
}

function renderStatus(data) {
  // Device & DMI
  if (data.dmi) {
    const model = data.dmi.product_family || data.dmi.product_name || "Legion Pro 5 16ADR10";
    const bios = data.dmi.bios_version || "RLCN33WW";
    document.getElementById("device-model").textContent = model;
    document.getElementById("bios-version").textContent = `BIOS: ${bios} · Linux Native`;
  }

  // Telemetry (CPU / RAM / GPU)
  if (data.telemetry) {
    const { cpu, ram, gpu } = data.telemetry;

    // CPU
    if (cpu) {
      document.getElementById("cpu-usage").textContent = Math.round(cpu.usage_percent);
      document.getElementById("cpu-clock").textContent = `${cpu.clock_ghz.toFixed(2)} GHz`;
      document.getElementById("cpu-temp").textContent = `${cpu.temperature_c} °C`;
      document.getElementById("cpu-fan").textContent = `${cpu.fan_rpm} RPM`;
      document.getElementById("cpu-progress").style.width = `${Math.min(100, Math.max(0, cpu.usage_percent))}%`;
    }

    // RAM
    if (ram) {
      document.getElementById("ram-usage-pct").textContent = Math.round(ram.percent);
      document.getElementById("ram-used").textContent = `${ram.used_gb} GB`;
      document.getElementById("ram-total").textContent = `${ram.total_gb} GB`;
      document.getElementById("ram-progress").style.width = `${Math.min(100, Math.max(0, ram.percent))}%`;
    }

    // GPU
    if (gpu) {
      document.getElementById("gpu-usage").textContent = Math.round(gpu.usage_percent);
      const clockStr = gpu.clock_ghz > 0 ? `${gpu.clock_ghz.toFixed(2)} GHz` : `${gpu.clock_mhz} MHz`;
      document.getElementById("gpu-clock").textContent = `${clockStr} · ${gpu.power_w} W`;
      document.getElementById("gpu-temp").textContent = `${gpu.temperature_c} °C`;
      document.getElementById("gpu-fan").textContent = `${gpu.fan_rpm} RPM`;
      document.getElementById("gpu-progress").style.width = `${Math.min(100, Math.max(0, gpu.usage_percent))}%`;
    }
  }

  // Thermal Profile
  if (data.thermal && data.thermal.current_profile) {
    const prof = data.thermal.current_profile;
    updateActiveProfileButton(prof);

    const ledLabel = data.thermal.power_led_color || "White (Balanced)";
    document.getElementById("led-label").textContent = `LED: ${ledLabel}`;
    document.getElementById("header-profile").textContent = capitalize(prof);

    // Sync LED color
    const ledLight = document.getElementById("led-light");
    const headerDot = document.getElementById("header-profile-dot");
    const dotClass = `dot-${prof === "max-power" ? "perf" : prof}`;
    ledLight.className = "led-light";
    headerDot.className = "status-dot";
    if (prof === "quiet") {
      ledLight.style.background = "var(--color-quiet)";
      headerDot.style.background = "var(--color-quiet)";
    } else if (prof === "performance" || prof === "max-power") {
      ledLight.style.background = "var(--color-perf)";
      headerDot.style.background = "var(--color-perf)";
    } else if (prof === "custom") {
      ledLight.style.background = "var(--color-custom)";
      headerDot.style.background = "var(--color-custom)";
    } else {
      ledLight.style.background = "var(--color-balanced)";
      headerDot.style.background = "var(--color-balanced)";
    }
  }

  // Fans
  if (data.fans && data.fans.fans && data.fans.fans.length > 0) {
    const f1 = data.fans.fans[0];
    const f2 = data.fans.fans[1] || f1;
    const avgRpm = Math.round((f1.current_rpm + f2.current_rpm) / 2);
    const isManual = f1.is_manual;
    document.getElementById("fan-summary-text").textContent = `${isManual ? "Manual" : "Auto"} · ${avgRpm} RPM`;

    // Active pill state
    document.getElementById("btn-fan-auto").classList.toggle("active", !isManual && avgRpm < 4900);
    document.getElementById("btn-fan-full").classList.toggle("active", avgRpm >= 4900);
    document.getElementById("btn-fan-manual").classList.toggle("active", isManual);
  }

  // Battery
  if (data.battery) {
    const batt = data.battery;
    document.getElementById("header-battery").textContent = `${batt.capacity_percent}%`;
    document.getElementById("batt-pct").textContent = `${batt.capacity_percent}%`;
    document.getElementById("batt-status").textContent = batt.status || "Normal";
    document.getElementById("batt-cycles").textContent = batt.cycle_count || "266";
    document.getElementById("battery-health-badge").textContent = `${batt.health_percent.toFixed(1)}% Health`;

    // Switches (only set if not actively interacting)
    const swConserv = document.getElementById("switch-conservation");
    if (document.activeElement !== swConserv) {
      swConserv.checked = !!batt.conservation_mode;
    }
    const swUsb = document.getElementById("switch-usb-charge");
    if (document.activeElement !== swUsb) {
      swUsb.checked = !!batt.usb_charging;
    }
  }

  // Lighting & Fn-Lock
  if (data.lighting) {
    const swFn = document.getElementById("switch-fn-lock");
    if (document.activeElement !== swFn) {
      swFn.checked = !!data.lighting.fn_lock;
    }
  }
}

// Thermal Profile Switcher
function setupProfileButtons() {
  const buttons = document.querySelectorAll(".profile-btn");
  buttons.forEach(btn => {
    btn.addEventListener("click", async () => {
      const profile = btn.dataset.profile;
      updateActiveProfileButton(profile);
      try {
        const res = await fetch("/api/profile", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ profile }),
        });
        const data = await res.json();
        showToast(`Thermal mode switched to ${capitalize(profile)}`);
        fetchStatus();
      } catch (err) {
        showToast(`Error switching profile: ${err}`, true);
      }
    });
  });
}

function updateActiveProfileButton(profile) {
  const activeProfile = profile === "max-power" ? "performance" : profile;
  document.querySelectorAll(".profile-btn").forEach(btn => {
    btn.classList.toggle("active", btn.dataset.profile === activeProfile);
  });
}

// Fan Controls
async function setFanMode(mode) {
  try {
    let body = { mode };
    if (mode === "set") {
      const rpm = parseInt(document.getElementById("fan-slider").value);
      body.rpm = rpm;
    }
    const res = await fetch("/api/fan", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const data = await res.json();
    showToast(`Fan mode updated: ${capitalize(mode)}`);
    fetchStatus();
  } catch (err) {
    showToast(`Error setting fan mode: ${err}`, true);
  }
}

function onFanSliderChange(val) {
  document.getElementById("fan-target-display").textContent = `${val} RPM`;
  clearTimeout(fanSliderDebounceTimer);
  fanSliderDebounceTimer = setTimeout(() => {
    setFanMode("set");
  }, 400);
}

// Battery Toggles
async function toggleConservation(enabled) {
  try {
    const res = await fetch("/api/battery/conservation", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled }),
    });
    showToast(`Conservation mode ${enabled ? "enabled (capped at 80%)" : "disabled"}`);
    fetchStatus();
  } catch (err) {
    showToast(`Error toggling conservation mode: ${err}`, true);
  }
}

async function toggleUsbCharge(enabled) {
  try {
    const res = await fetch("/api/battery/usb-charge", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled }),
    });
    showToast(`Always-On USB charging ${enabled ? "enabled" : "disabled"}`);
    fetchStatus();
  } catch (err) {
    showToast(`Error toggling USB charging: ${err}`, true);
  }
}

// Lighting Presets & Custom Colors
async function applyPreset(preset) {
  try {
    const res = await fetch("/api/lighting/preset", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ preset }),
    });
    showToast(`Lighting preset applied: ${capitalize(preset.replace("-", " "))}`);

    // Update keyboard graphic preview
    updateKeyboardColorsForPreset(preset);
  } catch (err) {
    showToast(`Error applying preset: ${err}`, true);
  }
}

function updateKeyboardColorsForPreset(preset) {
  const z1 = document.getElementById("kbd-z1");
  const z2 = document.getElementById("kbd-z2");
  const z3 = document.getElementById("kbd-z3");
  const z4 = document.getElementById("kbd-z4");

  if (preset === "legion-blue") {
    [z1, z2, z3, z4].forEach(z => {
      z.style.boxShadow = "0 0 16px rgba(0, 211, 254, 0.4)";
      z.style.borderColor = "#00d3fe";
    });
  } else if (preset === "cyberpunk") {
    z1.style.borderColor = "#00d3fe"; z1.style.boxShadow = "0 0 16px rgba(0, 211, 254, 0.4)";
    z2.style.borderColor = "#ff007f"; z2.style.boxShadow = "0 0 16px rgba(255, 0, 127, 0.4)";
    z3.style.borderColor = "#00d3fe"; z3.style.boxShadow = "0 0 16px rgba(0, 211, 254, 0.4)";
    z4.style.borderColor = "#ff007f"; z4.style.boxShadow = "0 0 16px rgba(255, 0, 127, 0.4)";
  } else if (preset === "crimson") {
    [z1, z2, z3, z4].forEach(z => {
      z.style.boxShadow = "0 0 16px rgba(255, 42, 42, 0.4)";
      z.style.borderColor = "#ff2a2a";
    });
  } else if (preset === "stealth") {
    [z1, z2, z3, z4].forEach(z => {
      z.style.boxShadow = "none";
      z.style.borderColor = "transparent";
    });
  }
}

function selectZone(zone) {
  currentZoneTarget = zone;
  document.querySelectorAll(".zone-pill").forEach(p => {
    p.classList.toggle("active", p.id === `zp-${zone}`);
  });
  document.querySelectorAll(".kbd-zone").forEach(z => {
    z.classList.toggle("selected", zone === "all" || z.id === `kbd-z${zone}`);
  });
}

async function applyCustomColor(hex) {
  try {
    const res = await fetch("/api/lighting/static", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ zone: currentZoneTarget, color: hex, brightness: 3 }),
    });

    // Update keyboard graphic preview
    if (currentZoneTarget === "all") {
      ["kbd-z1", "kbd-z2", "kbd-z3", "kbd-z4"].forEach(id => {
        const el = document.getElementById(id);
        el.style.borderColor = hex;
        el.style.boxShadow = `0 0 16px ${hex}66`;
      });
    } else {
      const el = document.getElementById(`kbd-z${currentZoneTarget}`);
      if (el) {
        el.style.borderColor = hex;
        el.style.boxShadow = `0 0 16px ${hex}66`;
      }
    }
    showToast(`Zone ${currentZoneTarget.toUpperCase()} color updated`);
  } catch (err) {
    showToast(`Error setting color: ${err}`, true);
  }
}

async function toggleFnLock(enabled) {
  try {
    const res = await fetch("/api/lighting/fn-lock", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled }),
    });
    showToast(`Fn-Lock ${enabled ? "enabled" : "disabled"}`);
  } catch (err) {
    showToast(`Error toggling Fn-Lock: ${err}`, true);
  }
}

// Custom Mode Tuning
function applyCustomSetting(attribute, val, displayId, unit) {
  document.getElementById(displayId).textContent = `${val} ${unit}`;
  clearTimeout(customSliderDebounceTimer);
  customSliderDebounceTimer = setTimeout(async () => {
    try {
      const res = await fetch("/api/custom/set", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ attribute, value: parseInt(val) }),
      });
      showToast(`Custom setting updated: ${val} ${unit}`);
    } catch (err) {
      showToast(`Error updating custom limit: ${err}`, true);
    }
  }, 400);
}

// Toast Notifications Helper
function showToast(message, isError = false) {
  const container = document.getElementById("toast-container");
  const toast = document.createElement("div");
  toast.className = "toast";
  toast.innerHTML = `<span>${isError ? "⚠️" : "✓"}</span><span>${message}</span>`;
  container.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transform = "translateY(8px)";
    setTimeout(() => toast.remove(), 250);
  }, 3000);
}

function capitalize(s) {
  if (!s) return "";
  return s.charAt(0).toUpperCase() + s.slice(1);
}

