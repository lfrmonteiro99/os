/* ── Accessibility Manager ─────────────────────────── */
/* macOS-style accessibility: VoiceOver, font scaling, contrast, reduce motion */

class AccessibilityManager {
  constructor() {
    this.settings = {
      voiceOver: false,
      fontSize: 1.0,        // scale factor (0.5 – 3.0)
      highContrast: false,
      reduceMotion: false,
      reduceTransparency: false,
      invertColors: false,
      cursorSize: 1.0,       // scale factor (1.0 – 4.0)
      stickyKeys: false,
      slowKeys: false,
      slowKeysDelay: 300,    // ms
    };
    this.voiceOverQueue = [];
    this.listeners = [];
  }

  /* ── VoiceOver ────────────────── */
  enableVoiceOver() {
    this.settings.voiceOver = true;
    this._emit("voiceover", true);
  }

  disableVoiceOver() {
    this.settings.voiceOver = false;
    this.voiceOverQueue = [];
    this._emit("voiceover", false);
  }

  speak(text) {
    if (!this.settings.voiceOver) return null;
    var utterance = { text: text, timestamp: Date.now() };
    this.voiceOverQueue.push(utterance);
    return utterance;
  }

  getVoiceOverQueue() {
    return this.voiceOverQueue.slice();
  }

  clearVoiceOverQueue() {
    this.voiceOverQueue = [];
  }

  /* ── Font Scaling ─────────────── */
  setFontSize(scale) {
    if (scale < 0.5) scale = 0.5;
    if (scale > 3.0) scale = 3.0;
    this.settings.fontSize = Math.round(scale * 100) / 100;
    this._emit("fontsize", this.settings.fontSize);
    return this.settings.fontSize;
  }

  increaseFontSize(step) {
    step = step || 0.1;
    return this.setFontSize(this.settings.fontSize + step);
  }

  decreaseFontSize(step) {
    step = step || 0.1;
    return this.setFontSize(this.settings.fontSize - step);
  }

  resetFontSize() {
    return this.setFontSize(1.0);
  }

  /* ── Display Settings ─────────── */
  toggleHighContrast() {
    this.settings.highContrast = !this.settings.highContrast;
    this._emit("highcontrast", this.settings.highContrast);
    return this.settings.highContrast;
  }

  toggleReduceMotion() {
    this.settings.reduceMotion = !this.settings.reduceMotion;
    this._emit("reducemotion", this.settings.reduceMotion);
    return this.settings.reduceMotion;
  }

  toggleReduceTransparency() {
    this.settings.reduceTransparency = !this.settings.reduceTransparency;
    this._emit("reducetransparency", this.settings.reduceTransparency);
    return this.settings.reduceTransparency;
  }

  toggleInvertColors() {
    this.settings.invertColors = !this.settings.invertColors;
    this._emit("invertcolors", this.settings.invertColors);
    return this.settings.invertColors;
  }

  /* ── Cursor ───────────────────── */
  setCursorSize(scale) {
    if (scale < 1.0) scale = 1.0;
    if (scale > 4.0) scale = 4.0;
    this.settings.cursorSize = Math.round(scale * 10) / 10;
    this._emit("cursorsize", this.settings.cursorSize);
    return this.settings.cursorSize;
  }

  /* ── Keyboard ─────────────────── */
  toggleStickyKeys() {
    this.settings.stickyKeys = !this.settings.stickyKeys;
    this._emit("stickykeys", this.settings.stickyKeys);
    return this.settings.stickyKeys;
  }

  toggleSlowKeys() {
    this.settings.slowKeys = !this.settings.slowKeys;
    this._emit("slowkeys", this.settings.slowKeys);
    return this.settings.slowKeys;
  }

  setSlowKeysDelay(ms) {
    if (ms < 100) ms = 100;
    if (ms > 2000) ms = 2000;
    this.settings.slowKeysDelay = ms;
    return ms;
  }

  /* ── Settings Export / Import ──── */
  getSettings() {
    return Object.assign({}, this.settings);
  }

  applySettings(obj) {
    var self = this;
    Object.keys(obj).forEach(function (k) {
      if (k in self.settings) self.settings[k] = obj[k];
    });
    this._emit("settingschanged", this.getSettings());
  }

  exportJSON() {
    return JSON.stringify(this.settings);
  }

  importJSON(json) {
    var parsed = JSON.parse(json);
    this.applySettings(parsed);
    return this.getSettings();
  }

  /* ── Events ───────────────────── */
  on(event, fn) {
    this.listeners.push({ event: event, fn: fn });
  }

  _emit(event, data) {
    this.listeners.forEach(function (l) {
      if (l.event === event) l.fn(data);
    });
  }
}

if (typeof module !== "undefined") module.exports = { AccessibilityManager: AccessibilityManager };
