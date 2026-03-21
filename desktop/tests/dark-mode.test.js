const { DarkModeManager } = require("../modules/dark-mode");

describe("DarkModeManager", () => {
  let dm;
  beforeEach(() => { dm = new DarkModeManager(); });

  /* ── Constructor Defaults ────── */
  describe("Constructor Defaults", () => {
    test("default mode is light", () => {
      expect(dm.mode).toBe("light");
    });

    test("default autoSwitch is false", () => {
      expect(dm.autoSwitch).toBe(false);
    });

    test("default schedule has sunrise 07:00 and sunset 20:00", () => {
      expect(dm.schedule).toEqual({ sunrise: "07:00", sunset: "20:00" });
    });

    test("default accent color is blue", () => {
      expect(dm.accentColor).toBe("blue");
    });

    test("default wallpaper tinting is true", () => {
      expect(dm.wallpaperTinting).toBe(true);
    });

    test("default contrast level is normal", () => {
      expect(dm.contrastLevel).toBe("normal");
    });

    test("default reduceTransparency is false", () => {
      expect(dm.reduceTransparency).toBe(false);
    });

    test("constructor accepts custom options", () => {
      const custom = new DarkModeManager({
        mode: "dark",
        autoSwitch: true,
        schedule: { sunrise: "06:00", sunset: "19:00" },
        accentColor: "purple",
      });
      expect(custom.mode).toBe("dark");
      expect(custom.autoSwitch).toBe(true);
      expect(custom.schedule).toEqual({ sunrise: "06:00", sunset: "19:00" });
      expect(custom.accentColor).toBe("purple");
    });
  });

  /* ── Mode Switching ──────────── */
  describe("Mode Switching", () => {
    test("setMode switches to dark", () => {
      dm.setMode("dark");
      expect(dm.mode).toBe("dark");
    });

    test("setMode switches to light", () => {
      dm.setMode("dark");
      dm.setMode("light");
      expect(dm.mode).toBe("light");
    });

    test("toggle switches from light to dark", () => {
      dm.toggle();
      expect(dm.mode).toBe("dark");
    });

    test("toggle switches from dark to light", () => {
      dm.setMode("dark");
      dm.toggle();
      expect(dm.mode).toBe("light");
    });

    test("setMode rejects invalid modes", () => {
      expect(() => dm.setMode("blue")).toThrow();
    });
  });

  /* ── Schedule-Based Auto Switching ── */
  describe("Schedule-Based Auto Switching", () => {
    test("setSchedule updates sunrise and sunset", () => {
      dm.setSchedule("06:30", "19:30");
      expect(dm.schedule).toEqual({ sunrise: "06:30", sunset: "19:30" });
    });

    test("enableAutoSwitch turns on auto switching", () => {
      dm.enableAutoSwitch();
      expect(dm.autoSwitch).toBe(true);
    });

    test("disableAutoSwitch turns off auto switching", () => {
      dm.enableAutoSwitch();
      dm.disableAutoSwitch();
      expect(dm.autoSwitch).toBe(false);
    });

    test("getModeForTime returns dark outside schedule", () => {
      dm.setSchedule("07:00", "20:00");
      expect(dm.getModeForTime("22:00")).toBe("dark");
      expect(dm.getModeForTime("05:00")).toBe("dark");
    });

    test("getModeForTime returns light inside schedule", () => {
      dm.setSchedule("07:00", "20:00");
      expect(dm.getModeForTime("12:00")).toBe("light");
      expect(dm.getModeForTime("07:00")).toBe("light");
    });
  });

  /* ── Accent Color Management ─── */
  describe("Accent Color Management", () => {
    test("setAccentColor accepts valid colors", () => {
      const validColors = ["blue", "purple", "pink", "red", "orange", "yellow", "green", "graphite"];
      validColors.forEach((color) => {
        dm.setAccentColor(color);
        expect(dm.accentColor).toBe(color);
      });
    });

    test("setAccentColor rejects invalid colors", () => {
      expect(() => dm.setAccentColor("magenta")).toThrow();
    });

    test("getAccentColor returns current accent", () => {
      dm.setAccentColor("pink");
      expect(dm.getAccentColor()).toBe("pink");
    });
  });

  /* ── Wallpaper Tinting ────────── */
  describe("Wallpaper Tinting", () => {
    test("enableWallpaperTinting sets tinting to true", () => {
      dm.wallpaperTinting = false;
      dm.enableWallpaperTinting();
      expect(dm.wallpaperTinting).toBe(true);
    });

    test("disableWallpaperTinting sets tinting to false", () => {
      dm.disableWallpaperTinting();
      expect(dm.wallpaperTinting).toBe(false);
    });
  });

  /* ── Custom Color Schemes ────── */
  describe("Custom Color Schemes", () => {
    test("registerScheme adds a custom scheme", () => {
      dm.registerScheme("solarized", { bg: "#002b36", text: "#839496" });
      expect(dm.customSchemes.has("solarized")).toBe(true);
    });

    test("applyScheme sets mode to custom and stores active scheme", () => {
      dm.registerScheme("solarized", { bg: "#002b36", text: "#839496" });
      dm.applyScheme("solarized");
      expect(dm.mode).toBe("custom");
      expect(dm.activeScheme).toBe("solarized");
    });

    test("applyScheme throws for unregistered scheme", () => {
      expect(() => dm.applyScheme("nonexistent")).toThrow();
    });

    test("removeScheme deletes a custom scheme", () => {
      dm.registerScheme("solarized", { bg: "#002b36", text: "#839496" });
      dm.removeScheme("solarized");
      expect(dm.customSchemes.has("solarized")).toBe(false);
    });
  });

  /* ── Per-App Overrides ────────── */
  describe("Per-App Overrides", () => {
    test("setAppOverride stores an app override", () => {
      dm.setAppOverride("Safari", "dark");
      expect(dm.appOverrides.get("Safari")).toBe("dark");
    });

    test("getAppOverride returns the override for an app", () => {
      dm.setAppOverride("Mail", "light");
      expect(dm.getAppOverride("Mail")).toBe("light");
    });

    test("getAppOverride returns null for unset apps", () => {
      expect(dm.getAppOverride("Notes")).toBeNull();
    });

    test("removeAppOverride deletes the override", () => {
      dm.setAppOverride("Safari", "dark");
      dm.removeAppOverride("Safari");
      expect(dm.getAppOverride("Safari")).toBeNull();
    });
  });

  /* ── onChange Listeners ────────── */
  describe("onChange Listeners", () => {
    test("onChange fires when mode changes", () => {
      let received = null;
      dm.onChange((data) => { received = data; });
      dm.setMode("dark");
      expect(received).toEqual({ property: "mode", value: "dark" });
    });

    test("onChange fires on toggle", () => {
      let received = null;
      dm.onChange((data) => { received = data; });
      dm.toggle();
      expect(received).toEqual({ property: "mode", value: "dark" });
    });

    test("multiple listeners all fire", () => {
      let count = 0;
      dm.onChange(() => { count++; });
      dm.onChange(() => { count++; });
      dm.setMode("dark");
      expect(count).toBe(2);
    });
  });

  /* ── getCurrentColors ──────────── */
  describe("getCurrentColors", () => {
    test("returns light palette by default", () => {
      const colors = dm.getCurrentColors();
      expect(colors.bg).toBe("#ffffff");
      expect(colors.text).toBe("#1d1d1f");
    });

    test("returns dark palette when in dark mode", () => {
      dm.setMode("dark");
      const colors = dm.getCurrentColors();
      expect(colors.bg).toBe("#1e1e1e");
      expect(colors.text).toBe("#f5f5f7");
    });

    test("returns custom scheme colors when active", () => {
      dm.registerScheme("mono", { bg: "#000000", text: "#ffffff" });
      dm.applyScheme("mono");
      const colors = dm.getCurrentColors();
      expect(colors.bg).toBe("#000000");
      expect(colors.text).toBe("#ffffff");
    });
  });

  /* ── Contrast Level ────────────── */
  describe("Contrast Level", () => {
    test("setContrastLevel to high", () => {
      dm.setContrastLevel("high");
      expect(dm.contrastLevel).toBe("high");
    });

    test("setContrastLevel to normal", () => {
      dm.setContrastLevel("high");
      dm.setContrastLevel("normal");
      expect(dm.contrastLevel).toBe("normal");
    });

    test("setContrastLevel rejects invalid values", () => {
      expect(() => dm.setContrastLevel("extreme")).toThrow();
    });
  });

  /* ── Reduce Transparency ───────── */
  describe("Reduce Transparency", () => {
    test("toggleReduceTransparency flips state", () => {
      expect(dm.toggleReduceTransparency()).toBe(true);
      expect(dm.toggleReduceTransparency()).toBe(false);
    });
  });

  /* ── Export/Import Preferences ── */
  describe("Export/Import Preferences", () => {
    test("exportPreferences returns a JSON string", () => {
      const json = dm.exportPreferences();
      const parsed = JSON.parse(json);
      expect(parsed.mode).toBe("light");
      expect(parsed.accentColor).toBe("blue");
    });

    test("importPreferences restores state", () => {
      dm.setMode("dark");
      dm.setAccentColor("red");
      dm.setContrastLevel("high");
      const json = dm.exportPreferences();

      const fresh = new DarkModeManager();
      fresh.importPreferences(json);
      expect(fresh.mode).toBe("dark");
      expect(fresh.accentColor).toBe("red");
      expect(fresh.contrastLevel).toBe("high");
    });

    test("importPreferences roundtrip preserves schedule", () => {
      dm.setSchedule("05:00", "21:00");
      dm.enableAutoSwitch();
      const json = dm.exportPreferences();

      const fresh = new DarkModeManager();
      fresh.importPreferences(json);
      expect(fresh.schedule).toEqual({ sunrise: "05:00", sunset: "21:00" });
      expect(fresh.autoSwitch).toBe(true);
    });
  });
});
