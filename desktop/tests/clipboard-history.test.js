/**
 * TDD Tests for Clipboard History Manager (Issue #13)
 * RED phase: write tests before implementation
 */
const { ClipboardHistory } = require('../modules/clipboard-history');

describe('ClipboardHistory', () => {
  let clipboard;

  beforeEach(() => {
    clipboard = new ClipboardHistory({ maxSize: 50 });
  });

  describe('constructor', () => {
    test('initializes with empty history', () => {
      expect(clipboard.getAll()).toEqual([]);
      expect(clipboard.size()).toBe(0);
    });

    test('respects custom maxSize', () => {
      const small = new ClipboardHistory({ maxSize: 5 });
      expect(small.maxSize).toBe(5);
    });

    test('defaults maxSize to 50', () => {
      const def = new ClipboardHistory();
      expect(def.maxSize).toBe(50);
    });
  });

  describe('copy()', () => {
    test('adds text entry to history', () => {
      clipboard.copy('Hello World', 'text', 'Notes');
      expect(clipboard.size()).toBe(1);
      const entry = clipboard.getAll()[0];
      expect(entry.content).toBe('Hello World');
      expect(entry.type).toBe('text');
      expect(entry.source).toBe('Notes');
    });

    test('adds entry at the front (most recent first)', () => {
      clipboard.copy('First', 'text', 'Notes');
      clipboard.copy('Second', 'text', 'Safari');
      expect(clipboard.getAll()[0].content).toBe('Second');
      expect(clipboard.getAll()[1].content).toBe('First');
    });

    test('includes timestamp on each entry', () => {
      clipboard.copy('Test', 'text', 'App');
      const entry = clipboard.getAll()[0];
      expect(entry.timestamp).toBeDefined();
      expect(typeof entry.timestamp).toBe('number');
    });

    test('moves duplicate to front instead of adding again', () => {
      clipboard.copy('Alpha', 'text', 'A');
      clipboard.copy('Beta', 'text', 'B');
      clipboard.copy('Alpha', 'text', 'C');
      expect(clipboard.size()).toBe(2);
      expect(clipboard.getAll()[0].content).toBe('Alpha');
      expect(clipboard.getAll()[0].source).toBe('C'); // updated source
    });

    test('enforces maxSize by dropping oldest entries', () => {
      const small = new ClipboardHistory({ maxSize: 3 });
      small.copy('A', 'text', 'x');
      small.copy('B', 'text', 'x');
      small.copy('C', 'text', 'x');
      small.copy('D', 'text', 'x');
      expect(small.size()).toBe(3);
      expect(small.getAll().map(e => e.content)).toEqual(['D', 'C', 'B']);
    });

    test('supports different content types', () => {
      clipboard.copy('/path/to/file', 'file', 'Finder');
      clipboard.copy('https://example.com', 'link', 'Safari');
      expect(clipboard.getAll()[0].type).toBe('link');
      expect(clipboard.getAll()[1].type).toBe('file');
    });

    test('does not add empty content', () => {
      clipboard.copy('', 'text', 'App');
      expect(clipboard.size()).toBe(0);
    });
  });

  describe('paste()', () => {
    test('returns the most recent entry content', () => {
      clipboard.copy('Hello', 'text', 'App');
      clipboard.copy('World', 'text', 'App');
      expect(clipboard.paste()).toBe('World');
    });

    test('returns null when history is empty', () => {
      expect(clipboard.paste()).toBeNull();
    });
  });

  describe('pasteAt(index)', () => {
    test('returns entry at specific index', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.copy('B', 'text', 'x');
      clipboard.copy('C', 'text', 'x');
      expect(clipboard.pasteAt(0)).toBe('C');
      expect(clipboard.pasteAt(2)).toBe('A');
    });

    test('returns null for out-of-range index', () => {
      clipboard.copy('A', 'text', 'x');
      expect(clipboard.pasteAt(5)).toBeNull();
      expect(clipboard.pasteAt(-1)).toBeNull();
    });
  });

  describe('pin() / unpin()', () => {
    test('pins an entry so it is not evicted', () => {
      const small = new ClipboardHistory({ maxSize: 3 });
      small.copy('Pinned', 'text', 'x');
      small.pin(0);
      small.copy('B', 'text', 'x');
      small.copy('C', 'text', 'x');
      small.copy('D', 'text', 'x');
      // Pinned entry should survive eviction
      const contents = small.getAll().map(e => e.content);
      expect(contents).toContain('Pinned');
    });

    test('getPinned() returns only pinned entries', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.copy('B', 'text', 'x');
      clipboard.pin(1); // pin A
      expect(clipboard.getPinned().length).toBe(1);
      expect(clipboard.getPinned()[0].content).toBe('A');
    });

    test('unpin removes pin status', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.pin(0);
      expect(clipboard.getAll()[0].pinned).toBe(true);
      clipboard.unpin(0);
      expect(clipboard.getAll()[0].pinned).toBe(false);
    });
  });

  describe('remove()', () => {
    test('removes entry at index', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.copy('B', 'text', 'x');
      clipboard.remove(0);
      expect(clipboard.size()).toBe(1);
      expect(clipboard.getAll()[0].content).toBe('A');
    });

    test('does nothing for invalid index', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.remove(5);
      expect(clipboard.size()).toBe(1);
    });
  });

  describe('clear()', () => {
    test('removes all non-pinned entries', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.copy('B', 'text', 'x');
      clipboard.pin(1); // pin A
      clipboard.clear();
      expect(clipboard.size()).toBe(1);
      expect(clipboard.getAll()[0].content).toBe('A');
    });

    test('clearAll() removes everything including pinned', () => {
      clipboard.copy('A', 'text', 'x');
      clipboard.pin(0);
      clipboard.clearAll();
      expect(clipboard.size()).toBe(0);
    });
  });

  describe('search()', () => {
    test('finds entries matching query', () => {
      clipboard.copy('Hello World', 'text', 'x');
      clipboard.copy('Goodbye Moon', 'text', 'x');
      clipboard.copy('Hello Again', 'text', 'x');
      const results = clipboard.search('Hello');
      expect(results.length).toBe(2);
      expect(results[0].content).toBe('Hello Again');
    });

    test('returns empty array for no match', () => {
      clipboard.copy('Alpha', 'text', 'x');
      expect(clipboard.search('xyz')).toEqual([]);
    });

    test('is case-insensitive', () => {
      clipboard.copy('HELLO', 'text', 'x');
      expect(clipboard.search('hello').length).toBe(1);
    });
  });
});
