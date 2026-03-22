/**
 * TDD Tests for Share Sheet System (Issue #56)
 * RED phase: write tests before implementation
 */
const { ShareSheet } = require('../modules/share-sheet');

describe('ShareSheet', () => {
  let share;

  beforeEach(() => {
    share = new ShareSheet();
  });

  describe('constructor', () => {
    test('starts with default targets', () => {
      const targets = share.getTargets();
      expect(targets.length).toBeGreaterThan(0);
    });

    test('is not visible initially', () => {
      expect(share.isVisible()).toBe(false);
    });
  });

  describe('default targets', () => {
    test('includes core share targets', () => {
      const names = share.getTargets().map(t => t.name);
      expect(names).toContain('Messages');
      expect(names).toContain('Mail');
      expect(names).toContain('Notes');
      expect(names).toContain('Copy Link');
    });
  });

  describe('registerTarget()', () => {
    test('adds a custom share target', () => {
      share.registerTarget({ name: 'Slack', icon: '💬', types: ['text', 'link'] });
      const names = share.getTargets().map(t => t.name);
      expect(names).toContain('Slack');
    });

    test('rejects target without name', () => {
      expect(() => share.registerTarget({ icon: '?', types: ['text'] })).toThrow('Target name is required');
    });

    test('rejects duplicate target name', () => {
      share.registerTarget({ name: 'Custom', icon: '📌', types: ['text'] });
      expect(() => share.registerTarget({ name: 'Custom', icon: '📌', types: ['text'] }))
        .toThrow('Target already exists');
    });
  });

  describe('removeTarget()', () => {
    test('removes a share target by name', () => {
      share.registerTarget({ name: 'Temp', icon: '🗑️', types: ['text'] });
      share.removeTarget('Temp');
      const names = share.getTargets().map(t => t.name);
      expect(names).not.toContain('Temp');
    });

    test('does nothing for nonexistent target', () => {
      const before = share.getTargets().length;
      share.removeTarget('Nonexistent');
      expect(share.getTargets().length).toBe(before);
    });
  });

  describe('share()', () => {
    test('shares text content to a target', () => {
      const handler = jest.fn();
      share.onShare(handler);
      const result = share.share('Messages', { type: 'text', content: 'Hello!' });
      expect(result.success).toBe(true);
      expect(handler).toHaveBeenCalledWith(
        expect.objectContaining({ name: 'Messages' }),
        expect.objectContaining({ type: 'text', content: 'Hello!' })
      );
    });

    test('shares link content', () => {
      const handler = jest.fn();
      share.onShare(handler);
      share.share('Mail', { type: 'link', content: 'https://example.com', title: 'Example' });
      expect(handler).toHaveBeenCalledWith(
        expect.objectContaining({ name: 'Mail' }),
        expect.objectContaining({ type: 'link', content: 'https://example.com' })
      );
    });

    test('fails for unknown target', () => {
      const result = share.share('Unknown', { type: 'text', content: 'Hi' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Target not found');
    });

    test('fails when content type not supported by target', () => {
      // Copy Link only supports link and text
      const result = share.share('Copy Link', { type: 'image', content: 'data:...' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Content type not supported');
    });

    test('stores share in history', () => {
      share.share('Messages', { type: 'text', content: 'Hello' });
      expect(share.getHistory().length).toBe(1);
      expect(share.getHistory()[0].target).toBe('Messages');
    });
  });

  describe('getTargetsForType()', () => {
    test('returns only targets that support the given type', () => {
      const textTargets = share.getTargetsForType('text');
      textTargets.forEach(t => {
        expect(t.types).toContain('text');
      });
    });

    test('returns empty array for unsupported type', () => {
      expect(share.getTargetsForType('unknown_format')).toEqual([]);
    });
  });

  describe('open() / close()', () => {
    test('open sets visible to true', () => {
      share.open({ type: 'text', content: 'Hello' });
      expect(share.isVisible()).toBe(true);
    });

    test('open stores the content to be shared', () => {
      share.open({ type: 'text', content: 'Test' });
      expect(share.getCurrentContent().content).toBe('Test');
    });

    test('close sets visible to false', () => {
      share.open({ type: 'text', content: 'x' });
      share.close();
      expect(share.isVisible()).toBe(false);
    });

    test('close clears current content', () => {
      share.open({ type: 'text', content: 'x' });
      share.close();
      expect(share.getCurrentContent()).toBeNull();
    });
  });

  describe('getHistory()', () => {
    test('returns share history newest first', () => {
      share.share('Messages', { type: 'text', content: 'First' });
      share.share('Mail', { type: 'text', content: 'Second' });
      expect(share.getHistory()[0].target).toBe('Mail');
    });

    test('clearHistory empties the history', () => {
      share.share('Messages', { type: 'text', content: 'Hi' });
      share.clearHistory();
      expect(share.getHistory()).toEqual([]);
    });
  });
});
