/**
 * TDD Tests for System Preferences (macOS-style preference panes)
 * RED phase: write tests before implementation
 */
const { SystemPreferences } = require('../modules/system-preferences');

describe('SystemPreferences', () => {
  let sp;

  beforeEach(() => {
    sp = new SystemPreferences();
  });

  describe('constructor', () => {
    test('initializes with default panes', () => {
      const panes = sp.listPanes();
      expect(panes.length).toBeGreaterThanOrEqual(10);
    });

    test('starts with no pane open', () => {
      expect(sp.getCurrentPane()).toBeNull();
    });

    test('starts with no listeners', () => {
      expect(sp.listeners.size).toBe(0);
    });
  });

  describe('listPanes()', () => {
    test('includes General pane', () => {
      expect(sp.listPanes()).toContain('General');
    });

    test('includes Desktop pane', () => {
      expect(sp.listPanes()).toContain('Desktop');
    });

    test('includes Dock pane', () => {
      expect(sp.listPanes()).toContain('Dock');
    });

    test('includes Display pane', () => {
      expect(sp.listPanes()).toContain('Display');
    });

    test('includes Sound pane', () => {
      expect(sp.listPanes()).toContain('Sound');
    });

    test('includes Keyboard pane', () => {
      expect(sp.listPanes()).toContain('Keyboard');
    });

    test('includes Mouse pane', () => {
      expect(sp.listPanes()).toContain('Mouse');
    });

    test('includes Network pane', () => {
      expect(sp.listPanes()).toContain('Network');
    });

    test('includes Users pane', () => {
      expect(sp.listPanes()).toContain('Users');
    });

    test('includes Security pane', () => {
      expect(sp.listPanes()).toContain('Security');
    });
  });

  describe('openPane() / closePane() / getCurrentPane()', () => {
    test('opens a pane by name', () => {
      sp.openPane('General');
      expect(sp.getCurrentPane()).toBe('General');
    });

    test('closes the current pane', () => {
      sp.openPane('General');
      sp.closePane();
      expect(sp.getCurrentPane()).toBeNull();
    });

    test('throws when opening unknown pane', () => {
      expect(() => sp.openPane('NonExistent')).toThrow('Unknown pane');
    });

    test('switching panes changes current pane', () => {
      sp.openPane('General');
      sp.openPane('Dock');
      expect(sp.getCurrentPane()).toBe('Dock');
    });
  });

  describe('searchPanes()', () => {
    test('finds panes by keyword match', () => {
      const results = sp.searchPanes('volume');
      expect(results).toContain('Sound');
    });

    test('returns empty array for no match', () => {
      const results = sp.searchPanes('xyznonexistent');
      expect(results).toEqual([]);
    });

    test('search is case insensitive', () => {
      const results = sp.searchPanes('WALLPAPER');
      expect(results).toContain('Desktop');
    });
  });

  describe('General settings', () => {
    test('default appearance is light', () => {
      expect(sp.getSetting('General', 'appearance')).toBe('light');
    });

    test('default accent color is blue', () => {
      expect(sp.getSetting('General', 'accentColor')).toBe('blue');
    });

    test('default sidebar icon size is medium', () => {
      expect(sp.getSetting('General', 'sidebarIconSize')).toBe('medium');
    });

    test('default scroll bar behavior is automatic', () => {
      expect(sp.getSetting('General', 'scrollBarBehavior')).toBe('automatic');
    });
  });

  describe('Desktop settings', () => {
    test('default wallpaper path', () => {
      expect(sp.getSetting('Desktop', 'wallpaperPath')).toBe('/System/Library/Desktop Pictures/default.jpg');
    });

    test('default screen saver name', () => {
      expect(sp.getSetting('Desktop', 'screenSaverName')).toBe('Flurry');
    });

    test('default screen saver timeout', () => {
      expect(sp.getSetting('Desktop', 'screenSaverTimeout')).toBe(300);
    });
  });

  describe('Dock settings', () => {
    test('default dock size', () => {
      expect(sp.getSetting('Dock', 'size')).toBe(48);
    });

    test('default magnification', () => {
      expect(sp.getSetting('Dock', 'magnification')).toBe(false);
    });

    test('default position is bottom', () => {
      expect(sp.getSetting('Dock', 'position')).toBe('bottom');
    });

    test('default auto-hide is false', () => {
      expect(sp.getSetting('Dock', 'autoHide')).toBe(false);
    });

    test('default animation is genie', () => {
      expect(sp.getSetting('Dock', 'animation')).toBe('genie');
    });
  });

  describe('Display settings', () => {
    test('default resolution', () => {
      expect(sp.getSetting('Display', 'resolution')).toBe('2560x1600');
    });

    test('default brightness', () => {
      expect(sp.getSetting('Display', 'brightness')).toBe(75);
    });

    test('default nightShift is false', () => {
      expect(sp.getSetting('Display', 'nightShift')).toBe(false);
    });

    test('default trueTone is true', () => {
      expect(sp.getSetting('Display', 'trueTone')).toBe(true);
    });
  });

  describe('Sound settings', () => {
    test('default output volume', () => {
      expect(sp.getSetting('Sound', 'outputVolume')).toBe(50);
    });

    test('default input volume', () => {
      expect(sp.getSetting('Sound', 'inputVolume')).toBe(50);
    });

    test('default output device', () => {
      expect(sp.getSetting('Sound', 'outputDevice')).toBe('Internal Speakers');
    });

    test('default input device', () => {
      expect(sp.getSetting('Sound', 'inputDevice')).toBe('Internal Microphone');
    });

    test('default alert sound', () => {
      expect(sp.getSetting('Sound', 'alertSound')).toBe('Tink');
    });
  });

  describe('Keyboard settings', () => {
    test('default key repeat rate', () => {
      expect(sp.getSetting('Keyboard', 'keyRepeatRate')).toBe(6);
    });

    test('default delay until repeat', () => {
      expect(sp.getSetting('Keyboard', 'delayUntilRepeat')).toBe(2);
    });

    test('default shortcuts is a map object', () => {
      var shortcuts = sp.getSetting('Keyboard', 'shortcuts');
      expect(typeof shortcuts).toBe('object');
      expect(shortcuts['copy']).toBe('Cmd+C');
      expect(shortcuts['paste']).toBe('Cmd+V');
    });
  });

  describe('setSetting() / getSetting()', () => {
    test('sets and gets a setting value', () => {
      sp.setSetting('General', 'appearance', 'dark');
      expect(sp.getSetting('General', 'appearance')).toBe('dark');
    });

    test('throws for unknown pane', () => {
      expect(() => sp.setSetting('FakePane', 'key', 'val')).toThrow('Unknown pane');
    });

    test('throws for unknown key', () => {
      expect(() => sp.setSetting('General', 'fakeKey', 'val')).toThrow('Unknown setting');
    });

    test('getSetting throws for unknown pane', () => {
      expect(() => sp.getSetting('FakePane', 'key')).toThrow('Unknown pane');
    });

    test('getSetting throws for unknown key', () => {
      expect(() => sp.getSetting('General', 'fakeKey')).toThrow('Unknown setting');
    });
  });

  describe('resetPane()', () => {
    test('resets pane settings to defaults', () => {
      sp.setSetting('Sound', 'outputVolume', 80);
      sp.setSetting('Sound', 'inputVolume', 30);
      sp.resetPane('Sound');
      expect(sp.getSetting('Sound', 'outputVolume')).toBe(50);
      expect(sp.getSetting('Sound', 'inputVolume')).toBe(50);
    });

    test('throws for unknown pane', () => {
      expect(() => sp.resetPane('FakePane')).toThrow('Unknown pane');
    });
  });

  describe('exportSettings() / importSettings()', () => {
    test('exports all settings as JSON string', () => {
      var json = sp.exportSettings();
      expect(typeof json).toBe('string');
      var parsed = JSON.parse(json);
      expect(parsed['General']).toBeDefined();
      expect(parsed['Sound']).toBeDefined();
    });

    test('imports settings from JSON string', () => {
      sp.setSetting('Sound', 'outputVolume', 99);
      var json = sp.exportSettings();
      var sp2 = new SystemPreferences();
      sp2.importSettings(json);
      expect(sp2.getSetting('Sound', 'outputVolume')).toBe(99);
    });

    test('import merges with existing panes', () => {
      sp.setSetting('General', 'appearance', 'dark');
      var json = sp.exportSettings();
      var sp2 = new SystemPreferences();
      sp2.importSettings(json);
      expect(sp2.getSetting('General', 'appearance')).toBe('dark');
      expect(sp2.getSetting('Dock', 'size')).toBe(48);
    });
  });

  describe('onChange()', () => {
    test('registers a listener for a pane', () => {
      var called = false;
      sp.onChange('Sound', function () { called = true; });
      sp.setSetting('Sound', 'outputVolume', 80);
      expect(called).toBe(true);
    });

    test('listener receives pane, key, and value', () => {
      var received = {};
      sp.onChange('Sound', function (pane, key, value) {
        received = { pane: pane, key: key, value: value };
      });
      sp.setSetting('Sound', 'outputVolume', 70);
      expect(received.pane).toBe('Sound');
      expect(received.key).toBe('outputVolume');
      expect(received.value).toBe(70);
    });

    test('multiple listeners on same pane all fire', () => {
      var count = 0;
      sp.onChange('Sound', function () { count++; });
      sp.onChange('Sound', function () { count++; });
      sp.setSetting('Sound', 'outputVolume', 60);
      expect(count).toBe(2);
    });

    test('listener does not fire for other panes', () => {
      var called = false;
      sp.onChange('Sound', function () { called = true; });
      sp.setSetting('General', 'appearance', 'dark');
      expect(called).toBe(false);
    });
  });

  describe('validation', () => {
    test('rejects volume below 0', () => {
      expect(() => sp.setSetting('Sound', 'outputVolume', -1)).toThrow('Invalid value');
    });

    test('rejects volume above 100', () => {
      expect(() => sp.setSetting('Sound', 'outputVolume', 101)).toThrow('Invalid value');
    });

    test('rejects invalid dock position', () => {
      expect(() => sp.setSetting('Dock', 'position', 'top')).toThrow('Invalid value');
    });

    test('accepts valid dock positions', () => {
      sp.setSetting('Dock', 'position', 'left');
      expect(sp.getSetting('Dock', 'position')).toBe('left');
      sp.setSetting('Dock', 'position', 'right');
      expect(sp.getSetting('Dock', 'position')).toBe('right');
      sp.setSetting('Dock', 'position', 'bottom');
      expect(sp.getSetting('Dock', 'position')).toBe('bottom');
    });

    test('rejects brightness below 0', () => {
      expect(() => sp.setSetting('Display', 'brightness', -5)).toThrow('Invalid value');
    });

    test('rejects brightness above 100', () => {
      expect(() => sp.setSetting('Display', 'brightness', 150)).toThrow('Invalid value');
    });

    test('rejects invalid appearance value', () => {
      expect(() => sp.setSetting('General', 'appearance', 'neon')).toThrow('Invalid value');
    });
  });
});
