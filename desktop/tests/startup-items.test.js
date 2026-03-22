/**
 * TDD Tests for Startup Items / Login Items Manager (Issue #57)
 * RED phase: write tests before implementation
 */
const { StartupManager } = require('../modules/startup-items');

describe('StartupManager', () => {
  let sm;

  beforeEach(() => {
    sm = new StartupManager();
  });

  describe('constructor', () => {
    test('starts with no startup items', () => {
      expect(sm.getAll()).toEqual([]);
      expect(sm.count()).toBe(0);
    });
  });

  describe('addItem()', () => {
    test('adds a login item', () => {
      sm.addItem({ name: 'Safari', type: 'app', enabled: true });
      expect(sm.count()).toBe(1);
      expect(sm.getAll()[0].name).toBe('Safari');
    });

    test('assigns unique id', () => {
      sm.addItem({ name: 'A', type: 'app', enabled: true });
      sm.addItem({ name: 'B', type: 'app', enabled: true });
      const ids = sm.getAll().map(i => i.id);
      expect(new Set(ids).size).toBe(2);
    });

    test('supports item types: app, background, agent', () => {
      sm.addItem({ name: 'App', type: 'app', enabled: true });
      sm.addItem({ name: 'BG', type: 'background', enabled: true });
      sm.addItem({ name: 'Agent', type: 'agent', enabled: true });
      expect(sm.getAll().map(i => i.type)).toEqual(['app', 'background', 'agent']);
    });

    test('defaults enabled to true', () => {
      sm.addItem({ name: 'Test', type: 'app' });
      expect(sm.getAll()[0].enabled).toBe(true);
    });

    test('supports hidden flag', () => {
      sm.addItem({ name: 'Daemon', type: 'background', hidden: true });
      expect(sm.getAll()[0].hidden).toBe(true);
    });

    test('rejects item without name', () => {
      expect(() => sm.addItem({ type: 'app' })).toThrow('Item name is required');
    });

    test('rejects invalid type', () => {
      expect(() => sm.addItem({ name: 'X', type: 'invalid' })).toThrow('Invalid item type');
    });

    test('rejects duplicate name', () => {
      sm.addItem({ name: 'Safari', type: 'app' });
      expect(() => sm.addItem({ name: 'Safari', type: 'app' })).toThrow('Item already exists');
    });
  });

  describe('removeItem()', () => {
    test('removes item by id', () => {
      sm.addItem({ name: 'Test', type: 'app' });
      const id = sm.getAll()[0].id;
      sm.removeItem(id);
      expect(sm.count()).toBe(0);
    });

    test('does nothing for unknown id', () => {
      sm.addItem({ name: 'X', type: 'app' });
      sm.removeItem(999);
      expect(sm.count()).toBe(1);
    });
  });

  describe('toggleItem()', () => {
    test('toggles enabled status', () => {
      sm.addItem({ name: 'Safari', type: 'app', enabled: true });
      const id = sm.getAll()[0].id;
      sm.toggleItem(id);
      expect(sm.getItem(id).enabled).toBe(false);
      sm.toggleItem(id);
      expect(sm.getItem(id).enabled).toBe(true);
    });
  });

  describe('getItem()', () => {
    test('returns item by id', () => {
      sm.addItem({ name: 'Test', type: 'app' });
      const id = sm.getAll()[0].id;
      expect(sm.getItem(id).name).toBe('Test');
    });

    test('returns null for unknown id', () => {
      expect(sm.getItem(999)).toBeNull();
    });
  });

  describe('getByType()', () => {
    test('filters by type', () => {
      sm.addItem({ name: 'A', type: 'app' });
      sm.addItem({ name: 'B', type: 'background' });
      sm.addItem({ name: 'C', type: 'app' });
      expect(sm.getByType('app').length).toBe(2);
      expect(sm.getByType('background').length).toBe(1);
    });
  });

  describe('getEnabled()', () => {
    test('returns only enabled items', () => {
      sm.addItem({ name: 'A', type: 'app', enabled: true });
      sm.addItem({ name: 'B', type: 'app', enabled: false });
      sm.addItem({ name: 'C', type: 'background', enabled: true });
      expect(sm.getEnabled().length).toBe(2);
    });
  });

  describe('reorder()', () => {
    test('moves item to new position', () => {
      sm.addItem({ name: 'A', type: 'app' });
      sm.addItem({ name: 'B', type: 'app' });
      sm.addItem({ name: 'C', type: 'app' });
      const idC = sm.getAll()[2].id;
      sm.reorder(idC, 0); // move C to first position
      expect(sm.getAll()[0].name).toBe('C');
    });
  });

  describe('getLaunchOrder()', () => {
    test('returns enabled items with stagger delays', () => {
      sm.addItem({ name: 'A', type: 'app' });
      sm.addItem({ name: 'B', type: 'app' });
      sm.addItem({ name: 'C', type: 'app', enabled: false });
      const order = sm.getLaunchOrder();
      expect(order.length).toBe(2);
      expect(order[0].delay).toBe(0);
      expect(order[1].delay).toBe(500);
    });

    test('configurable stagger delay', () => {
      sm.addItem({ name: 'A', type: 'app' });
      sm.addItem({ name: 'B', type: 'app' });
      const order = sm.getLaunchOrder(1000);
      expect(order[1].delay).toBe(1000);
    });
  });

  describe('updateItem()', () => {
    test('updates item fields', () => {
      sm.addItem({ name: 'Old', type: 'app' });
      const id = sm.getAll()[0].id;
      sm.updateItem(id, { name: 'New', hidden: true });
      expect(sm.getItem(id).name).toBe('New');
      expect(sm.getItem(id).hidden).toBe(true);
    });

    test('throws for unknown id', () => {
      expect(() => sm.updateItem(999, { name: 'X' })).toThrow('Item not found');
    });
  });

  describe('import/export', () => {
    test('export returns serializable array', () => {
      sm.addItem({ name: 'A', type: 'app' });
      sm.addItem({ name: 'B', type: 'background', hidden: true });
      const exported = sm.export();
      expect(typeof exported).toBe('string');
      const parsed = JSON.parse(exported);
      expect(parsed.length).toBe(2);
    });

    test('import restores items from JSON', () => {
      sm.addItem({ name: 'A', type: 'app' });
      const data = sm.export();
      const sm2 = new StartupManager();
      sm2.import(data);
      expect(sm2.count()).toBe(1);
      expect(sm2.getAll()[0].name).toBe('A');
    });
  });
});
