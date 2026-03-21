/* ── Dark Mode Manager ───────────────────────────── */
/* macOS-style dark mode: mode switching, scheduling, accent colors, custom schemes */

var VALID_ACCENT_COLORS = ["blue", "purple", "pink", "red", "orange", "yellow", "green", "graphite"];
var VALID_CONTRAST_LEVELS = ["normal", "high"];

class DarkModeManager {
  constructor(options) {
    options = options || {};
    this.mode = options.mode || "light";
    this.autoSwitch = options.autoSwitch || false;
    this.schedule = options.schedule || { sunrise: "07:00", sunset: "20:00" };
    this.accentColor = options.accentColor || "blue";
    this.wallpaperTinting = true;
    this.reduceTransparency = false;
    this.contrastLevel = "normal";
    this.listeners = [];
    this.appOverrides = new Map();
    this.customSchemes = new Map();
    this.activeScheme = null;
    this.colors = {
      light: { bg: "#ffffff", text: "#1d1d1f", secondary: "#86868b", accent: "#007aff", surface: "#f5f5f7", border: "#d2d2d7" },
      dark: { bg: "#1e1e1e", text: "#f5f5f7", secondary: "#98989d", accent: "#0a84ff", surface: "#2c2c2e", border: "#38383a" }
    };
  }

  /* ── Mode Switching ─────────────── */
  setMode(mode) {
    if (mode !== "light" && mode !== "dark") {
      throw new Error("Invalid mode: " + mode + ". Must be 'light' or 'dark'.");
    }
    this.mode = mode;
    this.activeScheme = null;
    this._emit({ property: "mode", value: mode });
  }

  toggle() {
    this.setMode(this.mode === "light" ? "dark" : "light");
  }

  /* ── Schedule ───────────────────── */
  setSchedule(sunrise, sunset) {
    this.schedule = { sunrise: sunrise, sunset: sunset };
    this._emit({ property: "schedule", value: this.schedule });
  }

  enableAutoSwitch() {
    this.autoSwitch = true;
    this._emit({ property: "autoSwitch", value: true });
  }

  disableAutoSwitch() {
    this.autoSwitch = false;
    this._emit({ property: "autoSwitch", value: false });
  }

  getModeForTime(timeStr) {
    var toMinutes = function (t) {
      var parts = t.split(":");
      return parseInt(parts[0], 10) * 60 + parseInt(parts[1], 10);
    };
    var current = toMinutes(timeStr);
    var sunrise = toMinutes(this.schedule.sunrise);
    var sunset = toMinutes(this.schedule.sunset);
    if (current >= sunrise && current < sunset) {
      return "light";
    }
    return "dark";
  }

  /* ── Accent Color ───────────────── */
  setAccentColor(color) {
    if (VALID_ACCENT_COLORS.indexOf(color) === -1) {
      throw new Error("Invalid accent color: " + color);
    }
    this.accentColor = color;
    this._emit({ property: "accentColor", value: color });
  }

  getAccentColor() {
    return this.accentColor;
  }

  /* ── Wallpaper Tinting ──────────── */
  enableWallpaperTinting() {
    this.wallpaperTinting = true;
    this._emit({ property: "wallpaperTinting", value: true });
  }

  disableWallpaperTinting() {
    this.wallpaperTinting = false;
    this._emit({ property: "wallpaperTinting", value: false });
  }

  /* ── Custom Color Schemes ───────── */
  registerScheme(name, colors) {
    this.customSchemes.set(name, colors);
  }

  applyScheme(name) {
    if (!this.customSchemes.has(name)) {
      throw new Error("Scheme not found: " + name);
    }
    this.mode = "custom";
    this.activeScheme = name;
    this._emit({ property: "scheme", value: name });
  }

  removeScheme(name) {
    this.customSchemes.delete(name);
    if (this.activeScheme === name) {
      this.activeScheme = null;
      this.mode = "light";
    }
  }

  /* ── Per-App Overrides ──────────── */
  setAppOverride(appName, mode) {
    this.appOverrides.set(appName, mode);
  }

  getAppOverride(appName) {
    if (this.appOverrides.has(appName)) {
      return this.appOverrides.get(appName);
    }
    return null;
  }

  removeAppOverride(appName) {
    this.appOverrides.delete(appName);
  }

  /* ── getCurrentColors ───────────── */
  getCurrentColors() {
    if (this.mode === "custom" && this.activeScheme && this.customSchemes.has(this.activeScheme)) {
      return Object.assign({}, this.customSchemes.get(this.activeScheme));
    }
    if (this.mode === "dark") {
      return Object.assign({}, this.colors.dark);
    }
    return Object.assign({}, this.colors.light);
  }

  /* ── Contrast Level ─────────────── */
  setContrastLevel(level) {
    if (VALID_CONTRAST_LEVELS.indexOf(level) === -1) {
      throw new Error("Invalid contrast level: " + level + ". Must be 'normal' or 'high'.");
    }
    this.contrastLevel = level;
    this._emit({ property: "contrastLevel", value: level });
  }

  /* ── Reduce Transparency ────────── */
  toggleReduceTransparency() {
    this.reduceTransparency = !this.reduceTransparency;
    this._emit({ property: "reduceTransparency", value: this.reduceTransparency });
    return this.reduceTransparency;
  }

  /* ── Export/Import Preferences ──── */
  exportPreferences() {
    return JSON.stringify({
      mode: this.mode,
      autoSwitch: this.autoSwitch,
      schedule: this.schedule,
      accentColor: this.accentColor,
      wallpaperTinting: this.wallpaperTinting,
      reduceTransparency: this.reduceTransparency,
      contrastLevel: this.contrastLevel
    });
  }

  importPreferences(json) {
    var prefs = JSON.parse(json);
    if (prefs.mode !== undefined) this.mode = prefs.mode;
    if (prefs.autoSwitch !== undefined) this.autoSwitch = prefs.autoSwitch;
    if (prefs.schedule !== undefined) this.schedule = prefs.schedule;
    if (prefs.accentColor !== undefined) this.accentColor = prefs.accentColor;
    if (prefs.wallpaperTinting !== undefined) this.wallpaperTinting = prefs.wallpaperTinting;
    if (prefs.reduceTransparency !== undefined) this.reduceTransparency = prefs.reduceTransparency;
    if (prefs.contrastLevel !== undefined) this.contrastLevel = prefs.contrastLevel;
    this._emit({ property: "preferences", value: prefs });
  }

  /* ── Events ─────────────────────── */
  onChange(fn) {
    this.listeners.push(fn);
  }

  _emit(data) {
    this.listeners.forEach(function (fn) {
      fn(data);
    });
  }
}

if (typeof module !== "undefined") module.exports = { DarkModeManager: DarkModeManager };
