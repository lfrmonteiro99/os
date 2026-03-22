/**
 * Screenshot & Screen Recording Manager (Issue #12)
 * Supports fullscreen, selection, window capture, and recording.
 */

class ScreenshotManager {
  constructor() {
    this._captures = [];
    this._mode = null;
    this._nextId = 1;
    this._recording = false;
    this._recordingStart = null;
  }

  getMode() {
    return this._mode;
  }

  setMode(mode) {
    var valid = ['fullscreen', 'selection', 'window'];
    if (valid.indexOf(mode) === -1) {
      throw new Error('Invalid screenshot mode: ' + mode);
    }
    this._mode = mode;
  }

  cancel() {
    this._mode = null;
  }

  captureFullscreen(width, height) {
    var capture = {
      id: this._nextId++,
      type: 'fullscreen',
      x: 0,
      y: 0,
      width: width,
      height: height,
      filename: this._generateFilename(),
      timestamp: Date.now(),
    };
    this._captures.unshift(capture);
    this._mode = null;
    return capture;
  }

  captureSelection(x, y, w, h) {
    // Normalize negative dimensions
    if (w < 0) { x = x + w; w = -w; }
    if (h < 0) { y = y + h; h = -h; }
    if (w === 0 && h === 0) {
      throw new Error('Invalid selection: zero size');
    }
    var capture = {
      id: this._nextId++,
      type: 'selection',
      x: x,
      y: y,
      width: w,
      height: h,
      filename: this._generateFilename(),
      timestamp: Date.now(),
    };
    this._captures.unshift(capture);
    this._mode = null;
    return capture;
  }

  captureWindow(win) {
    if (!win) throw new Error('No window specified');
    var capture = {
      id: this._nextId++,
      type: 'window',
      windowTitle: win.title,
      x: win.x,
      y: win.y,
      width: win.width,
      height: win.height,
      filename: this._generateFilename(),
      timestamp: Date.now(),
    };
    this._captures.unshift(capture);
    this._mode = null;
    return capture;
  }

  getCaptures() {
    return this._captures.slice();
  }

  deleteCapture(id) {
    this._captures = this._captures.filter(function (c) { return c.id !== id; });
  }

  clearCaptures() {
    this._captures = [];
  }

  getShortcuts() {
    return {
      fullscreen: 'Ctrl+Shift+3',
      selection: 'Ctrl+Shift+4',
      window: 'Ctrl+Shift+4+Space',
    };
  }

  getModeForShortcut(shortcut) {
    var map = {
      'Ctrl+Shift+3': 'fullscreen',
      'Ctrl+Shift+4': 'selection',
      'Ctrl+Shift+4+Space': 'window',
    };
    return map[shortcut] || null;
  }

  startRecording() {
    if (this._recording) throw new Error('Already recording');
    this._recording = true;
    this._recordingStart = Date.now();
  }

  stopRecording() {
    if (!this._recording) throw new Error('Not currently recording');
    var duration = Date.now() - this._recordingStart;
    this._recording = false;
    this._recordingStart = null;
    var capture = {
      id: this._nextId++,
      type: 'recording',
      duration: duration,
      filename: this._generateFilename(),
      timestamp: Date.now(),
    };
    this._captures.unshift(capture);
    return capture;
  }

  isRecording() {
    return this._recording;
  }

  _generateFilename() {
    var now = new Date();
    var pad = function (n) { return String(n).padStart(2, '0'); };
    return 'Screenshot_' +
      now.getFullYear() + '-' + pad(now.getMonth() + 1) + '-' + pad(now.getDate()) +
      '_' + pad(now.getHours()) + '-' + pad(now.getMinutes()) + '-' + pad(now.getSeconds());
  }
}

module.exports = { ScreenshotManager };
