/**
 * TDD Tests for Screenshot & Screen Recording Utility (Issue #12)
 * RED phase: write tests before implementation
 */
const { ScreenshotManager } = require('../modules/screenshot');

describe('ScreenshotManager', () => {
  let sm;

  beforeEach(() => {
    sm = new ScreenshotManager();
  });

  describe('constructor', () => {
    test('initializes with no captures', () => {
      expect(sm.getCaptures()).toEqual([]);
    });

    test('defaults mode to null (inactive)', () => {
      expect(sm.getMode()).toBeNull();
    });
  });

  describe('setMode()', () => {
    test('accepts fullscreen mode', () => {
      sm.setMode('fullscreen');
      expect(sm.getMode()).toBe('fullscreen');
    });

    test('accepts selection mode', () => {
      sm.setMode('selection');
      expect(sm.getMode()).toBe('selection');
    });

    test('accepts window mode', () => {
      sm.setMode('window');
      expect(sm.getMode()).toBe('window');
    });

    test('rejects invalid mode', () => {
      expect(() => sm.setMode('invalid')).toThrow('Invalid screenshot mode');
    });

    test('cancel() sets mode back to null', () => {
      sm.setMode('fullscreen');
      sm.cancel();
      expect(sm.getMode()).toBeNull();
    });
  });

  describe('captureFullscreen()', () => {
    test('creates capture with full dimensions', () => {
      const capture = sm.captureFullscreen(1920, 1080);
      expect(capture.type).toBe('fullscreen');
      expect(capture.width).toBe(1920);
      expect(capture.height).toBe(1080);
      expect(capture.x).toBe(0);
      expect(capture.y).toBe(0);
    });

    test('generates filename with timestamp pattern', () => {
      const capture = sm.captureFullscreen(1920, 1080);
      expect(capture.filename).toMatch(/^Screenshot_\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}$/);
    });

    test('stores capture in history', () => {
      sm.captureFullscreen(1920, 1080);
      expect(sm.getCaptures().length).toBe(1);
    });

    test('resets mode to null after capture', () => {
      sm.setMode('fullscreen');
      sm.captureFullscreen(1920, 1080);
      expect(sm.getMode()).toBeNull();
    });
  });

  describe('captureSelection()', () => {
    test('creates capture with specified rectangle', () => {
      const capture = sm.captureSelection(100, 200, 400, 300);
      expect(capture.type).toBe('selection');
      expect(capture.x).toBe(100);
      expect(capture.y).toBe(200);
      expect(capture.width).toBe(400);
      expect(capture.height).toBe(300);
    });

    test('rejects zero-size selection', () => {
      expect(() => sm.captureSelection(100, 100, 0, 0)).toThrow('Invalid selection');
    });

    test('normalizes negative dimensions (drag up-left)', () => {
      const capture = sm.captureSelection(500, 500, -200, -150);
      expect(capture.x).toBe(300);
      expect(capture.y).toBe(350);
      expect(capture.width).toBe(200);
      expect(capture.height).toBe(150);
    });
  });

  describe('captureWindow()', () => {
    test('creates capture from window bounds', () => {
      const win = { title: 'Safari', x: 50, y: 80, width: 600, height: 400 };
      const capture = sm.captureWindow(win);
      expect(capture.type).toBe('window');
      expect(capture.windowTitle).toBe('Safari');
      expect(capture.width).toBe(600);
    });

    test('rejects null window', () => {
      expect(() => sm.captureWindow(null)).toThrow('No window specified');
    });
  });

  describe('capture history', () => {
    test('getCaptures returns newest first', () => {
      sm.captureFullscreen(1920, 1080);
      sm.captureSelection(0, 0, 100, 100);
      expect(sm.getCaptures()[0].type).toBe('selection');
    });

    test('deleteCapture removes by id', () => {
      sm.captureFullscreen(1920, 1080);
      const id = sm.getCaptures()[0].id;
      sm.deleteCapture(id);
      expect(sm.getCaptures().length).toBe(0);
    });

    test('clearCaptures removes all', () => {
      sm.captureFullscreen(1920, 1080);
      sm.captureFullscreen(1920, 1080);
      sm.clearCaptures();
      expect(sm.getCaptures().length).toBe(0);
    });
  });

  describe('keyboard shortcuts', () => {
    test('getShortcuts returns mapping of keys to modes', () => {
      const shortcuts = sm.getShortcuts();
      expect(shortcuts).toEqual({
        'fullscreen': 'Ctrl+Shift+3',
        'selection': 'Ctrl+Shift+4',
        'window': 'Ctrl+Shift+4+Space',
      });
    });

    test('getModeForShortcut returns correct mode', () => {
      expect(sm.getModeForShortcut('Ctrl+Shift+3')).toBe('fullscreen');
      expect(sm.getModeForShortcut('Ctrl+Shift+4')).toBe('selection');
      expect(sm.getModeForShortcut('unknown')).toBeNull();
    });
  });

  describe('screen recording', () => {
    test('starts recording', () => {
      sm.startRecording();
      expect(sm.isRecording()).toBe(true);
    });

    test('stops recording and returns capture', () => {
      sm.startRecording();
      const result = sm.stopRecording();
      expect(sm.isRecording()).toBe(false);
      expect(result.type).toBe('recording');
      expect(result.duration).toBeDefined();
    });

    test('throws if stopping when not recording', () => {
      expect(() => sm.stopRecording()).toThrow('Not currently recording');
    });

    test('throws if starting when already recording', () => {
      sm.startRecording();
      expect(() => sm.startRecording()).toThrow('Already recording');
    });
  });
});
