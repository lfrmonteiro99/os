const { AccessibilityManager } = require("../modules/accessibility");

describe("AccessibilityManager", () => {
  let a11y;
  beforeEach(() => { a11y = new AccessibilityManager(); });

  /* ── VoiceOver ──────────────── */
  describe("VoiceOver", () => {
    test("starts disabled", () => {
      expect(a11y.settings.voiceOver).toBe(false);
    });

    test("enables VoiceOver", () => {
      a11y.enableVoiceOver();
      expect(a11y.settings.voiceOver).toBe(true);
    });

    test("disables VoiceOver and clears queue", () => {
      a11y.enableVoiceOver();
      a11y.speak("Hello");
      a11y.disableVoiceOver();
      expect(a11y.settings.voiceOver).toBe(false);
      expect(a11y.getVoiceOverQueue()).toHaveLength(0);
    });

    test("speak() queues utterance when enabled", () => {
      a11y.enableVoiceOver();
      const u = a11y.speak("Button pressed");
      expect(u).toHaveProperty("text", "Button pressed");
      expect(u).toHaveProperty("timestamp");
      expect(a11y.getVoiceOverQueue()).toHaveLength(1);
    });

    test("speak() returns null when disabled", () => {
      expect(a11y.speak("ignored")).toBeNull();
    });

    test("clearVoiceOverQueue empties the queue", () => {
      a11y.enableVoiceOver();
      a11y.speak("A"); a11y.speak("B");
      a11y.clearVoiceOverQueue();
      expect(a11y.getVoiceOverQueue()).toHaveLength(0);
    });
  });

  /* ── Font Scaling ───────────── */
  describe("Font Scaling", () => {
    test("default font size is 1.0", () => {
      expect(a11y.settings.fontSize).toBe(1.0);
    });

    test("setFontSize clamps between 0.5 and 3.0", () => {
      expect(a11y.setFontSize(0.1)).toBe(0.5);
      expect(a11y.setFontSize(5.0)).toBe(3.0);
      expect(a11y.setFontSize(1.5)).toBe(1.5);
    });

    test("increaseFontSize increments by step", () => {
      a11y.setFontSize(1.0);
      expect(a11y.increaseFontSize(0.25)).toBe(1.25);
    });

    test("decreaseFontSize decrements by step", () => {
      a11y.setFontSize(1.5);
      expect(a11y.decreaseFontSize(0.25)).toBe(1.25);
    });

    test("resetFontSize returns to 1.0", () => {
      a11y.setFontSize(2.0);
      expect(a11y.resetFontSize()).toBe(1.0);
    });
  });

  /* ── Display Settings ───────── */
  describe("Display Settings", () => {
    test("toggleHighContrast flips state", () => {
      expect(a11y.toggleHighContrast()).toBe(true);
      expect(a11y.toggleHighContrast()).toBe(false);
    });

    test("toggleReduceMotion flips state", () => {
      expect(a11y.toggleReduceMotion()).toBe(true);
      expect(a11y.settings.reduceMotion).toBe(true);
    });

    test("toggleReduceTransparency flips state", () => {
      expect(a11y.toggleReduceTransparency()).toBe(true);
      expect(a11y.settings.reduceTransparency).toBe(true);
    });

    test("toggleInvertColors flips state", () => {
      expect(a11y.toggleInvertColors()).toBe(true);
      expect(a11y.settings.invertColors).toBe(true);
    });
  });

  /* ── Cursor ─────────────────── */
  describe("Cursor Size", () => {
    test("setCursorSize clamps between 1.0 and 4.0", () => {
      expect(a11y.setCursorSize(0.5)).toBe(1.0);
      expect(a11y.setCursorSize(5.0)).toBe(4.0);
      expect(a11y.setCursorSize(2.5)).toBe(2.5);
    });
  });

  /* ── Keyboard ───────────────── */
  describe("Keyboard Accessibility", () => {
    test("toggleStickyKeys flips state", () => {
      expect(a11y.toggleStickyKeys()).toBe(true);
      expect(a11y.toggleStickyKeys()).toBe(false);
    });

    test("toggleSlowKeys flips state", () => {
      expect(a11y.toggleSlowKeys()).toBe(true);
    });

    test("setSlowKeysDelay clamps between 100-2000ms", () => {
      expect(a11y.setSlowKeysDelay(50)).toBe(100);
      expect(a11y.setSlowKeysDelay(3000)).toBe(2000);
      expect(a11y.setSlowKeysDelay(500)).toBe(500);
    });
  });

  /* ── Settings Export/Import ─── */
  describe("Settings", () => {
    test("getSettings returns a copy", () => {
      const s = a11y.getSettings();
      s.voiceOver = true;
      expect(a11y.settings.voiceOver).toBe(false);
    });

    test("applySettings merges provided keys", () => {
      a11y.applySettings({ highContrast: true, fontSize: 2.0 });
      expect(a11y.settings.highContrast).toBe(true);
      expect(a11y.settings.fontSize).toBe(2.0);
    });

    test("exportJSON/importJSON roundtrips", () => {
      a11y.setFontSize(1.8);
      a11y.toggleHighContrast();
      const json = a11y.exportJSON();
      const fresh = new AccessibilityManager();
      fresh.importJSON(json);
      expect(fresh.settings.fontSize).toBe(1.8);
      expect(fresh.settings.highContrast).toBe(true);
    });
  });

  /* ── Events ─────────────────── */
  describe("Event Listeners", () => {
    test("on() fires callback on event", () => {
      let received = null;
      a11y.on("voiceover", (val) => { received = val; });
      a11y.enableVoiceOver();
      expect(received).toBe(true);
    });

    test("fontsize event fires on change", () => {
      let size = null;
      a11y.on("fontsize", (val) => { size = val; });
      a11y.setFontSize(2.0);
      expect(size).toBe(2.0);
    });
  });
});
