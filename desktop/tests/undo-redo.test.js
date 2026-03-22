/**
 * TDD Tests for Global Undo/Redo System (Issue #65)
 * RED phase: write tests before implementation
 */
const { UndoManager, UndoStack } = require('../modules/undo-redo');

describe('UndoStack', () => {
  let stack;

  beforeEach(() => {
    stack = new UndoStack({ maxSize: 100 });
  });

  describe('constructor', () => {
    test('starts empty', () => {
      expect(stack.canUndo()).toBe(false);
      expect(stack.canRedo()).toBe(false);
    });

    test('respects custom maxSize', () => {
      const small = new UndoStack({ maxSize: 5 });
      expect(small.maxSize).toBe(5);
    });
  });

  describe('push()', () => {
    test('adds an action to the undo stack', () => {
      stack.push({ type: 'insert', data: 'Hello' });
      expect(stack.canUndo()).toBe(true);
    });

    test('clears redo stack on new push', () => {
      stack.push({ type: 'insert', data: 'A' });
      stack.undo();
      expect(stack.canRedo()).toBe(true);
      stack.push({ type: 'insert', data: 'B' });
      expect(stack.canRedo()).toBe(false);
    });

    test('evicts oldest when maxSize exceeded', () => {
      const small = new UndoStack({ maxSize: 3 });
      small.push({ type: 'a', data: '1' });
      small.push({ type: 'b', data: '2' });
      small.push({ type: 'c', data: '3' });
      small.push({ type: 'd', data: '4' });
      expect(small.undoSize()).toBe(3);
      // Oldest ('a') should be gone
      small.undo(); // d
      small.undo(); // c
      small.undo(); // b
      expect(small.canUndo()).toBe(false);
    });
  });

  describe('undo()', () => {
    test('returns the most recent action', () => {
      stack.push({ type: 'insert', data: 'Hello' });
      const action = stack.undo();
      expect(action.type).toBe('insert');
      expect(action.data).toBe('Hello');
    });

    test('makes redo available', () => {
      stack.push({ type: 'insert', data: 'X' });
      stack.undo();
      expect(stack.canRedo()).toBe(true);
    });

    test('returns null when nothing to undo', () => {
      expect(stack.undo()).toBeNull();
    });

    test('multiple undos in order (LIFO)', () => {
      stack.push({ type: 'a', data: '1' });
      stack.push({ type: 'b', data: '2' });
      stack.push({ type: 'c', data: '3' });
      expect(stack.undo().data).toBe('3');
      expect(stack.undo().data).toBe('2');
      expect(stack.undo().data).toBe('1');
      expect(stack.undo()).toBeNull();
    });
  });

  describe('redo()', () => {
    test('returns the last undone action', () => {
      stack.push({ type: 'insert', data: 'X' });
      stack.undo();
      const action = stack.redo();
      expect(action.data).toBe('X');
    });

    test('returns null when nothing to redo', () => {
      expect(stack.redo()).toBeNull();
    });

    test('redo then undo roundtrip', () => {
      stack.push({ type: 'a', data: '1' });
      stack.push({ type: 'b', data: '2' });
      stack.undo(); // undo b
      stack.undo(); // undo a
      stack.redo(); // redo a
      expect(stack.canUndo()).toBe(true);
      expect(stack.canRedo()).toBe(true);
      const last = stack.undo();
      expect(last.data).toBe('1');
    });
  });

  describe('clear()', () => {
    test('clears both undo and redo stacks', () => {
      stack.push({ type: 'a', data: '1' });
      stack.undo();
      stack.clear();
      expect(stack.canUndo()).toBe(false);
      expect(stack.canRedo()).toBe(false);
    });
  });

  describe('peek()', () => {
    test('returns next undo action without removing it', () => {
      stack.push({ type: 'a', data: '1' });
      expect(stack.peek().data).toBe('1');
      expect(stack.canUndo()).toBe(true); // still there
    });

    test('returns null when empty', () => {
      expect(stack.peek()).toBeNull();
    });
  });
});

describe('UndoManager', () => {
  let manager;

  beforeEach(() => {
    manager = new UndoManager();
  });

  describe('per-window stacks', () => {
    test('creates stack for new window id', () => {
      manager.push('win-1', { type: 'edit', data: 'text' });
      expect(manager.canUndo('win-1')).toBe(true);
    });

    test('separate stacks per window', () => {
      manager.push('win-1', { type: 'a', data: '1' });
      manager.push('win-2', { type: 'b', data: '2' });
      expect(manager.canUndo('win-1')).toBe(true);
      expect(manager.canUndo('win-2')).toBe(true);
      manager.undo('win-1');
      expect(manager.canUndo('win-1')).toBe(false);
      expect(manager.canUndo('win-2')).toBe(true);
    });
  });

  describe('undo / redo via manager', () => {
    test('undo delegates to correct window stack', () => {
      manager.push('win-1', { type: 'insert', data: 'Hello' });
      const action = manager.undo('win-1');
      expect(action.data).toBe('Hello');
    });

    test('redo delegates to correct window stack', () => {
      manager.push('win-1', { type: 'insert', data: 'X' });
      manager.undo('win-1');
      const action = manager.redo('win-1');
      expect(action.data).toBe('X');
    });

    test('undo on unknown window returns null', () => {
      expect(manager.undo('nonexistent')).toBeNull();
    });
  });

  describe('removeWindow()', () => {
    test('removes stack for a closed window', () => {
      manager.push('win-1', { type: 'x', data: 'y' });
      manager.removeWindow('win-1');
      expect(manager.canUndo('win-1')).toBe(false);
    });
  });

  describe('getWindowIds()', () => {
    test('returns list of active window ids', () => {
      manager.push('win-1', { type: 'a', data: '1' });
      manager.push('win-2', { type: 'b', data: '2' });
      expect(manager.getWindowIds().sort()).toEqual(['win-1', 'win-2']);
    });
  });
});
