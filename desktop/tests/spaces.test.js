/**
 * TDD Tests for Spaces (Virtual Desktops) Feature
 * RED phase: write tests before implementation
 */
const { SpacesManager } = require('../modules/spaces');

describe('SpacesManager', () => {
  let manager;

  beforeEach(() => {
    manager = new SpacesManager();
  });

  describe('constructor', () => {
    test('creates a default space named Desktop 1', () => {
      const spaces = manager.getSpaces();
      expect(spaces.length).toBe(1);
      expect(spaces[0].name).toBe('Desktop 1');
    });

    test('sets the default space as active', () => {
      const active = manager.getActiveSpace();
      expect(active).not.toBeNull();
      expect(active.name).toBe('Desktop 1');
    });

    test('accepts custom default name via options', () => {
      const custom = new SpacesManager({ defaultName: 'Main' });
      expect(custom.getActiveSpace().name).toBe('Main');
    });
  });

  describe('addSpace(name)', () => {
    test('creates a new virtual desktop and returns it', () => {
      const space = manager.addSpace('Desktop 2');
      expect(space.name).toBe('Desktop 2');
      expect(space.id).toBeDefined();
    });

    test('increases total space count', () => {
      manager.addSpace('Desktop 2');
      expect(manager.getSpaces().length).toBe(2);
    });

    test('assigns unique ids to each space', () => {
      const s1 = manager.addSpace('A');
      const s2 = manager.addSpace('B');
      expect(s1.id).not.toBe(s2.id);
    });

    test('rejects adding beyond maximum of 16 spaces', () => {
      for (let i = 2; i <= 16; i++) {
        manager.addSpace('Desktop ' + i);
      }
      expect(manager.getSpaces().length).toBe(16);
      expect(() => manager.addSpace('Desktop 17')).toThrow();
    });
  });

  describe('removeSpace(id)', () => {
    test('removes a space by id', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.removeSpace(s2.id);
      expect(manager.getSpaces().length).toBe(1);
    });

    test('cannot remove the last remaining space', () => {
      const active = manager.getActiveSpace();
      expect(() => manager.removeSpace(active.id)).toThrow();
    });

    test('redistributes windows to the adjacent space', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.moveWindowToSpace('win-1', s2.id);
      manager.moveWindowToSpace('win-2', s2.id);
      const firstSpaceId = manager.getSpaces()[0].id;
      manager.removeSpace(s2.id);
      const windows = manager.getWindowsInSpace(firstSpaceId);
      expect(windows).toContain('win-1');
      expect(windows).toContain('win-2');
    });

    test('switches to adjacent space if removed space was active', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.switchTo(s2.id);
      expect(manager.getActiveSpace().id).toBe(s2.id);
      const firstId = manager.getSpaces()[0].id;
      manager.removeSpace(s2.id);
      expect(manager.getActiveSpace().id).toBe(firstId);
    });
  });

  describe('switchTo(id)', () => {
    test('switches the active space', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.switchTo(s2.id);
      expect(manager.getActiveSpace().id).toBe(s2.id);
    });

    test('throws if space id does not exist', () => {
      expect(() => manager.switchTo(999)).toThrow();
    });
  });

  describe('getActiveSpace()', () => {
    test('returns the current active space object', () => {
      const active = manager.getActiveSpace();
      expect(active).toHaveProperty('id');
      expect(active).toHaveProperty('name');
    });
  });

  describe('getSpaces()', () => {
    test('returns all spaces in order', () => {
      manager.addSpace('Desktop 2');
      manager.addSpace('Desktop 3');
      const names = manager.getSpaces().map(s => s.name);
      expect(names).toEqual(['Desktop 1', 'Desktop 2', 'Desktop 3']);
    });
  });

  describe('moveWindowToSpace(windowId, spaceId)', () => {
    test('assigns a window to a target space', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.moveWindowToSpace('win-1', s2.id);
      expect(manager.getWindowsInSpace(s2.id)).toContain('win-1');
    });

    test('removes window from previous space', () => {
      const firstId = manager.getActiveSpace().id;
      const s2 = manager.addSpace('Desktop 2');
      manager.moveWindowToSpace('win-1', firstId);
      manager.moveWindowToSpace('win-1', s2.id);
      expect(manager.getWindowsInSpace(firstId)).not.toContain('win-1');
    });

    test('throws if target space does not exist', () => {
      expect(() => manager.moveWindowToSpace('win-1', 999)).toThrow();
    });
  });

  describe('getWindowsInSpace(spaceId)', () => {
    test('returns array of window ids in a space', () => {
      const firstId = manager.getActiveSpace().id;
      manager.moveWindowToSpace('win-1', firstId);
      manager.moveWindowToSpace('win-2', firstId);
      const windows = manager.getWindowsInSpace(firstId);
      expect(windows).toEqual(expect.arrayContaining(['win-1', 'win-2']));
      expect(windows.length).toBe(2);
    });

    test('returns empty array for space with no windows', () => {
      const s2 = manager.addSpace('Desktop 2');
      expect(manager.getWindowsInSpace(s2.id)).toEqual([]);
    });
  });

  describe('getSpaceForWindow(windowId)', () => {
    test('returns the space id a window belongs to', () => {
      const firstId = manager.getActiveSpace().id;
      manager.moveWindowToSpace('win-1', firstId);
      expect(manager.getSpaceForWindow('win-1')).toBe(firstId);
    });

    test('returns null for unknown window', () => {
      expect(manager.getSpaceForWindow('nonexistent')).toBeNull();
    });
  });

  describe('sticky windows (window on all spaces)', () => {
    test('setStickyWindow makes window visible on all spaces', () => {
      const firstId = manager.getActiveSpace().id;
      const s2 = manager.addSpace('Desktop 2');
      manager.moveWindowToSpace('win-1', firstId);
      manager.setStickyWindow('win-1', true);
      expect(manager.getWindowsInSpace(firstId)).toContain('win-1');
      expect(manager.getWindowsInSpace(s2.id)).toContain('win-1');
    });

    test('unsetting sticky removes window from other spaces', () => {
      const firstId = manager.getActiveSpace().id;
      const s2 = manager.addSpace('Desktop 2');
      manager.moveWindowToSpace('win-1', firstId);
      manager.setStickyWindow('win-1', true);
      manager.setStickyWindow('win-1', false);
      expect(manager.getWindowsInSpace(firstId)).toContain('win-1');
      expect(manager.getWindowsInSpace(s2.id)).not.toContain('win-1');
    });
  });

  describe('moveSpace(id, newIndex)', () => {
    test('reorders spaces', () => {
      const s2 = manager.addSpace('Desktop 2');
      const s3 = manager.addSpace('Desktop 3');
      manager.moveSpace(s3.id, 0);
      const names = manager.getSpaces().map(s => s.name);
      expect(names[0]).toBe('Desktop 3');
    });

    test('throws if space id is invalid', () => {
      expect(() => manager.moveSpace(999, 0)).toThrow();
    });

    test('clamps newIndex to valid range', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.moveSpace(s2.id, 100);
      const spaces = manager.getSpaces();
      expect(spaces[spaces.length - 1].id).toBe(s2.id);
    });
  });

  describe('space wallpaper', () => {
    test('setWallpaper and getWallpaper per space', () => {
      const firstId = manager.getActiveSpace().id;
      manager.setWallpaper(firstId, '/images/mountain.jpg');
      expect(manager.getWallpaper(firstId)).toBe('/images/mountain.jpg');
    });

    test('different spaces can have different wallpapers', () => {
      const firstId = manager.getActiveSpace().id;
      const s2 = manager.addSpace('Desktop 2');
      manager.setWallpaper(firstId, '/images/a.jpg');
      manager.setWallpaper(s2.id, '/images/b.jpg');
      expect(manager.getWallpaper(firstId)).toBe('/images/a.jpg');
      expect(manager.getWallpaper(s2.id)).toBe('/images/b.jpg');
    });
  });

  describe('switchToNext() / switchToPrevious()', () => {
    test('switchToNext cycles forward', () => {
      const s2 = manager.addSpace('Desktop 2');
      const s3 = manager.addSpace('Desktop 3');
      manager.switchToNext();
      expect(manager.getActiveSpace().id).toBe(s2.id);
      manager.switchToNext();
      expect(manager.getActiveSpace().id).toBe(s3.id);
    });

    test('switchToNext wraps around to first', () => {
      manager.addSpace('Desktop 2');
      manager.switchToNext();
      manager.switchToNext();
      expect(manager.getActiveSpace().name).toBe('Desktop 1');
    });

    test('switchToPrevious cycles backward', () => {
      const s2 = manager.addSpace('Desktop 2');
      manager.switchTo(s2.id);
      manager.switchToPrevious();
      expect(manager.getActiveSpace().name).toBe('Desktop 1');
    });

    test('switchToPrevious wraps around to last', () => {
      manager.addSpace('Desktop 2');
      manager.addSpace('Desktop 3');
      manager.switchToPrevious();
      expect(manager.getActiveSpace().name).toBe('Desktop 3');
    });
  });

  describe('renameSpace(id, newName)', () => {
    test('renames a space', () => {
      const firstId = manager.getActiveSpace().id;
      manager.renameSpace(firstId, 'Work');
      expect(manager.getActiveSpace().name).toBe('Work');
    });

    test('throws if space does not exist', () => {
      expect(() => manager.renameSpace(999, 'Nope')).toThrow();
    });
  });

  describe('onSwitch callback listener', () => {
    test('fires switch listener when switching spaces', () => {
      const s2 = manager.addSpace('Desktop 2');
      const calls = [];
      manager.onSwitch((fromId, toId) => {
        calls.push({ fromId, toId });
      });
      const firstId = manager.getSpaces()[0].id;
      manager.switchTo(s2.id);
      expect(calls.length).toBe(1);
      expect(calls[0].fromId).toBe(firstId);
      expect(calls[0].toId).toBe(s2.id);
    });

    test('does not fire when switching to already active space', () => {
      const calls = [];
      manager.onSwitch(() => calls.push(true));
      const firstId = manager.getActiveSpace().id;
      manager.switchTo(firstId);
      expect(calls.length).toBe(0);
    });
  });

  describe('fullscreen app creates its own space', () => {
    test('createFullscreenSpace creates a new space with the app window', () => {
      const space = manager.createFullscreenSpace('win-fs', 'Fullscreen App');
      expect(space.name).toBe('Fullscreen App');
      expect(manager.getWindowsInSpace(space.id)).toContain('win-fs');
      expect(manager.getActiveSpace().id).toBe(space.id);
    });
  });
});
