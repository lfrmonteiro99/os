/**
 * TDD Tests for Window Snapping System
 * RED phase: write tests before implementation
 */
const { WindowSnapping } = require('../modules/window-snapping');

describe('WindowSnapping', () => {
  let snapping;

  beforeEach(() => {
    snapping = new WindowSnapping({
      screenWidth: 1920,
      screenHeight: 1080,
      menuBarHeight: 25,
      dockHeight: 70,
      edgeThreshold: 20,
    });
  });

  describe('constructor', () => {
    test('stores screen dimensions', () => {
      expect(snapping.screenWidth).toBe(1920);
      expect(snapping.screenHeight).toBe(1080);
    });

    test('stores menuBarHeight and dockHeight', () => {
      expect(snapping.menuBarHeight).toBe(25);
      expect(snapping.dockHeight).toBe(70);
    });

    test('uses default values when no options provided', () => {
      const def = new WindowSnapping();
      expect(def.screenWidth).toBe(1440);
      expect(def.screenHeight).toBe(900);
      expect(def.menuBarHeight).toBe(25);
      expect(def.dockHeight).toBe(70);
      expect(def.edgeThreshold).toBe(20);
    });

    test('starts with no windows tracked', () => {
      expect(snapping.getSnappedWindows()).toEqual([]);
    });
  });

  describe('registerWindow()', () => {
    test('registers a window with position and size', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      const state = snapping.getWindowState('win-1');
      expect(state.x).toBe(100);
      expect(state.y).toBe(100);
      expect(state.width).toBe(400);
      expect(state.height).toBe(300);
      expect(state.snapped).toBe(false);
    });

    test('returns null state for unregistered window', () => {
      expect(snapping.getWindowState('nonexistent')).toBeNull();
    });
  });

  describe('snapLeft()', () => {
    test('snaps window to left half of usable area', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapLeft('win-1');
      const state = snapping.getWindowState('win-1');
      expect(state.x).toBe(0);
      expect(state.y).toBe(25);
      expect(state.width).toBe(960);
      expect(state.height).toBe(1080 - 25 - 70);
      expect(state.snapped).toBe(true);
      expect(state.snapZone).toBe('left');
    });
  });

  describe('snapRight()', () => {
    test('snaps window to right half of usable area', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapRight('win-1');
      const state = snapping.getWindowState('win-1');
      expect(state.x).toBe(960);
      expect(state.y).toBe(25);
      expect(state.width).toBe(960);
      expect(state.height).toBe(1080 - 25 - 70);
      expect(state.snapped).toBe(true);
      expect(state.snapZone).toBe('right');
    });
  });

  describe('snapTop()', () => {
    test('snaps window to top half of usable area', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapTop('win-1');
      const state = snapping.getWindowState('win-1');
      const usableHeight = 1080 - 25 - 70;
      expect(state.x).toBe(0);
      expect(state.y).toBe(25);
      expect(state.width).toBe(1920);
      expect(state.height).toBe(Math.floor(usableHeight / 2));
      expect(state.snapped).toBe(true);
      expect(state.snapZone).toBe('top');
    });
  });

  describe('snapBottom()', () => {
    test('snaps window to bottom half of usable area', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapBottom('win-1');
      const state = snapping.getWindowState('win-1');
      const usableHeight = 1080 - 25 - 70;
      const halfHeight = Math.floor(usableHeight / 2);
      expect(state.x).toBe(0);
      expect(state.y).toBe(25 + halfHeight);
      expect(state.width).toBe(1920);
      expect(state.height).toBe(usableHeight - halfHeight);
      expect(state.snapped).toBe(true);
      expect(state.snapZone).toBe('bottom');
    });
  });

  describe('quarter snapping', () => {
    test('snapTopLeft snaps to top-left quarter', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapTopLeft('win-1');
      const state = snapping.getWindowState('win-1');
      const usableHeight = 1080 - 25 - 70;
      expect(state.x).toBe(0);
      expect(state.y).toBe(25);
      expect(state.width).toBe(960);
      expect(state.height).toBe(Math.floor(usableHeight / 2));
      expect(state.snapZone).toBe('topLeft');
    });

    test('snapTopRight snaps to top-right quarter', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapTopRight('win-1');
      const state = snapping.getWindowState('win-1');
      const usableHeight = 1080 - 25 - 70;
      expect(state.x).toBe(960);
      expect(state.y).toBe(25);
      expect(state.width).toBe(960);
      expect(state.height).toBe(Math.floor(usableHeight / 2));
      expect(state.snapZone).toBe('topRight');
    });

    test('snapBottomLeft snaps to bottom-left quarter', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapBottomLeft('win-1');
      const state = snapping.getWindowState('win-1');
      const usableHeight = 1080 - 25 - 70;
      const halfHeight = Math.floor(usableHeight / 2);
      expect(state.x).toBe(0);
      expect(state.y).toBe(25 + halfHeight);
      expect(state.width).toBe(960);
      expect(state.height).toBe(usableHeight - halfHeight);
      expect(state.snapZone).toBe('bottomLeft');
    });

    test('snapBottomRight snaps to bottom-right quarter', () => {
      snapping.registerWindow('win-1', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapBottomRight('win-1');
      const state = snapping.getWindowState('win-1');
      const usableHeight = 1080 - 25 - 70;
      const halfHeight = Math.floor(usableHeight / 2);
      expect(state.x).toBe(960);
      expect(state.y).toBe(25 + halfHeight);
      expect(state.width).toBe(960);
      expect(state.height).toBe(usableHeight - halfHeight);
      expect(state.snapZone).toBe('bottomRight');
    });
  });

  describe('maximize()', () => {
    test('maximizes window to full usable area', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      snapping.maximize('win-1');
      const state = snapping.getWindowState('win-1');
      expect(state.x).toBe(0);
      expect(state.y).toBe(25);
      expect(state.width).toBe(1920);
      expect(state.height).toBe(1080 - 25 - 70);
      expect(state.snapped).toBe(true);
      expect(state.snapZone).toBe('maximized');
    });
  });

  describe('restore()', () => {
    test('restores window to original position and size after snap', () => {
      snapping.registerWindow('win-1', { x: 100, y: 150, width: 400, height: 300 });
      snapping.snapLeft('win-1');
      snapping.restore('win-1');
      const state = snapping.getWindowState('win-1');
      expect(state.x).toBe(100);
      expect(state.y).toBe(150);
      expect(state.width).toBe(400);
      expect(state.height).toBe(300);
      expect(state.snapped).toBe(false);
      expect(state.snapZone).toBeNull();
    });

    test('restore on unsnapped window does nothing', () => {
      snapping.registerWindow('win-1', { x: 100, y: 150, width: 400, height: 300 });
      snapping.restore('win-1');
      const state = snapping.getWindowState('win-1');
      expect(state.x).toBe(100);
      expect(state.y).toBe(150);
    });

    test('stores original dimensions before snap', () => {
      snapping.registerWindow('win-1', { x: 50, y: 75, width: 600, height: 450 });
      snapping.maximize('win-1');
      snapping.restore('win-1');
      const state = snapping.getWindowState('win-1');
      expect(state.width).toBe(600);
      expect(state.height).toBe(450);
      expect(state.x).toBe(50);
      expect(state.y).toBe(75);
    });
  });

  describe('getSnapZone()', () => {
    test('detects left edge zone', () => {
      expect(snapping.getSnapZone(5, 500)).toBe('left');
    });

    test('detects right edge zone', () => {
      expect(snapping.getSnapZone(1915, 500)).toBe('right');
    });

    test('detects top edge zone', () => {
      expect(snapping.getSnapZone(500, 5)).toBe('top');
    });

    test('detects bottom edge zone', () => {
      expect(snapping.getSnapZone(500, 1075)).toBe('bottom');
    });

    test('detects top-left corner zone', () => {
      expect(snapping.getSnapZone(5, 5)).toBe('topLeft');
    });

    test('detects top-right corner zone', () => {
      expect(snapping.getSnapZone(1915, 5)).toBe('topRight');
    });

    test('detects bottom-left corner zone', () => {
      expect(snapping.getSnapZone(5, 1075)).toBe('bottomLeft');
    });

    test('detects bottom-right corner zone', () => {
      expect(snapping.getSnapZone(1915, 1075)).toBe('bottomRight');
    });

    test('returns null when cursor is not near any edge', () => {
      expect(snapping.getSnapZone(500, 500)).toBeNull();
    });
  });

  describe('screen edge threshold configuration', () => {
    test('uses default threshold of 20px', () => {
      expect(snapping.edgeThreshold).toBe(20);
    });

    test('respects custom threshold', () => {
      const custom = new WindowSnapping({ edgeThreshold: 50 });
      expect(custom.getSnapZone(45, 500)).toBe('left');
    });

    test('cursor just outside threshold returns null', () => {
      expect(snapping.getSnapZone(25, 500)).toBeNull();
    });
  });

  describe('getPreviewRect()', () => {
    test('returns preview rect for left zone', () => {
      const rect = snapping.getPreviewRect('left');
      expect(rect.x).toBe(0);
      expect(rect.y).toBe(25);
      expect(rect.width).toBe(960);
      expect(rect.height).toBe(1080 - 25 - 70);
    });

    test('returns preview rect for maximized zone', () => {
      const rect = snapping.getPreviewRect('maximized');
      expect(rect.x).toBe(0);
      expect(rect.y).toBe(25);
      expect(rect.width).toBe(1920);
      expect(rect.height).toBe(1080 - 25 - 70);
    });

    test('returns null for invalid zone', () => {
      expect(snapping.getPreviewRect('invalid')).toBeNull();
    });
  });

  describe('multiple windows side-by-side', () => {
    test('two windows snapped left and right do not overlap', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      snapping.registerWindow('win-2', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapLeft('win-1');
      snapping.snapRight('win-2');
      const s1 = snapping.getWindowState('win-1');
      const s2 = snapping.getWindowState('win-2');
      expect(s1.x + s1.width).toBeLessThanOrEqual(s2.x);
    });
  });

  describe('unsnap on drag', () => {
    test('unsnap returns window to original size at new position', () => {
      snapping.registerWindow('win-1', { x: 100, y: 150, width: 400, height: 300 });
      snapping.snapLeft('win-1');
      snapping.unsnap('win-1', 500, 400);
      const state = snapping.getWindowState('win-1');
      expect(state.width).toBe(400);
      expect(state.height).toBe(300);
      expect(state.x).toBe(500);
      expect(state.y).toBe(400);
      expect(state.snapped).toBe(false);
      expect(state.snapZone).toBeNull();
    });
  });

  describe('prevent overlapping snaps', () => {
    test('snapping a second window to same zone displaces the first', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      snapping.registerWindow('win-2', { x: 200, y: 200, width: 400, height: 300 });
      snapping.snapLeft('win-1');
      snapping.snapLeft('win-2');
      const s1 = snapping.getWindowState('win-1');
      expect(s1.snapped).toBe(false);
    });
  });

  describe('cascade unsnapped windows', () => {
    test('cascadeUnsnapped arranges windows with offset', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      snapping.registerWindow('win-2', { x: 100, y: 100, width: 400, height: 300 });
      snapping.registerWindow('win-3', { x: 100, y: 100, width: 400, height: 300 });
      snapping.cascadeUnsnapped();
      const s1 = snapping.getWindowState('win-1');
      const s2 = snapping.getWindowState('win-2');
      const s3 = snapping.getWindowState('win-3');
      expect(s2.x).toBe(s1.x + 30);
      expect(s2.y).toBe(s1.y + 30);
      expect(s3.x).toBe(s2.x + 30);
      expect(s3.y).toBe(s2.y + 30);
    });
  });

  describe('getSnappedWindows()', () => {
    test('returns all currently snapped window ids', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      snapping.registerWindow('win-2', { x: 200, y: 200, width: 400, height: 300 });
      snapping.registerWindow('win-3', { x: 300, y: 300, width: 400, height: 300 });
      snapping.snapLeft('win-1');
      snapping.snapRight('win-2');
      const snapped = snapping.getSnappedWindows();
      expect(snapped.sort()).toEqual(['win-1', 'win-2']);
    });

    test('returns empty array when no windows are snapped', () => {
      snapping.registerWindow('win-1', { x: 100, y: 100, width: 400, height: 300 });
      expect(snapping.getSnappedWindows()).toEqual([]);
    });
  });
});
