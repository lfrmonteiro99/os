/**
 * TDD Tests for Enhanced Notification Grouping & Actions (Issue #52)
 * RED phase: write tests before implementation
 */
const { NotificationCenter } = require('../modules/notification-center');

describe('NotificationCenter', () => {
  let nc;

  beforeEach(() => {
    nc = new NotificationCenter();
  });

  describe('constructor', () => {
    test('starts with no notifications', () => {
      expect(nc.getAll()).toEqual([]);
      expect(nc.count()).toBe(0);
    });
  });

  describe('add()', () => {
    test('adds a notification with required fields', () => {
      nc.add({ app: 'Messages', title: 'New Message', body: 'Hey!', icon: '💬' });
      expect(nc.count()).toBe(1);
      const n = nc.getAll()[0];
      expect(n.app).toBe('Messages');
      expect(n.title).toBe('New Message');
      expect(n.body).toBe('Hey!');
      expect(n.read).toBe(false);
    });

    test('assigns unique id to each notification', () => {
      nc.add({ app: 'Mail', title: 'Email', body: 'Hi' });
      nc.add({ app: 'Mail', title: 'Email 2', body: 'Yo' });
      const ids = nc.getAll().map(n => n.id);
      expect(new Set(ids).size).toBe(2);
    });

    test('includes timestamp', () => {
      nc.add({ app: 'Calendar', title: 'Event', body: 'Meeting at 3pm' });
      expect(nc.getAll()[0].timestamp).toBeDefined();
    });

    test('supports action buttons', () => {
      nc.add({
        app: 'Messages',
        title: 'Chat',
        body: 'Hello',
        actions: ['Reply', 'Mark as Read', 'Mute'],
      });
      expect(nc.getAll()[0].actions).toEqual(['Reply', 'Mark as Read', 'Mute']);
    });

    test('newest notifications appear first', () => {
      nc.add({ app: 'A', title: 'First', body: '1' });
      nc.add({ app: 'B', title: 'Second', body: '2' });
      expect(nc.getAll()[0].title).toBe('Second');
    });
  });

  describe('getGrouped()', () => {
    test('groups notifications by app', () => {
      nc.add({ app: 'Messages', title: 'Msg 1', body: 'Hi' });
      nc.add({ app: 'Mail', title: 'Email 1', body: 'Hi' });
      nc.add({ app: 'Messages', title: 'Msg 2', body: 'Hey' });
      const groups = nc.getGrouped();
      expect(Object.keys(groups)).toContain('Messages');
      expect(Object.keys(groups)).toContain('Mail');
      expect(groups['Messages'].length).toBe(2);
      expect(groups['Mail'].length).toBe(1);
    });

    test('groups have notifications in newest-first order', () => {
      nc.add({ app: 'Messages', title: 'Old', body: '1' });
      nc.add({ app: 'Messages', title: 'New', body: '2' });
      const msgs = nc.getGrouped()['Messages'];
      expect(msgs[0].title).toBe('New');
    });
  });

  describe('getGroupSummary()', () => {
    test('returns count and latest notification per group', () => {
      nc.add({ app: 'Messages', title: 'Msg 1', body: 'Hi' });
      nc.add({ app: 'Messages', title: 'Msg 2', body: 'Hey' });
      nc.add({ app: 'Mail', title: 'Email', body: 'Yo' });
      const summary = nc.getGroupSummary();
      expect(summary).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ app: 'Messages', count: 2, latest: 'Msg 2' }),
          expect.objectContaining({ app: 'Mail', count: 1, latest: 'Email' }),
        ])
      );
    });
  });

  describe('markRead() / markAllRead()', () => {
    test('marks a specific notification as read', () => {
      nc.add({ app: 'Messages', title: 'Msg', body: 'Hi' });
      const id = nc.getAll()[0].id;
      nc.markRead(id);
      expect(nc.getAll()[0].read).toBe(true);
    });

    test('markAllRead sets all to read', () => {
      nc.add({ app: 'A', title: 'A1', body: '1' });
      nc.add({ app: 'B', title: 'B1', body: '2' });
      nc.markAllRead();
      nc.getAll().forEach(n => expect(n.read).toBe(true));
    });

    test('getUnreadCount returns correct count', () => {
      nc.add({ app: 'A', title: 'A1', body: '1' });
      nc.add({ app: 'B', title: 'B1', body: '2' });
      nc.markRead(nc.getAll()[0].id);
      expect(nc.getUnreadCount()).toBe(1);
    });
  });

  describe('dismiss() / dismissGroup()', () => {
    test('dismisses single notification by id', () => {
      nc.add({ app: 'Messages', title: 'Msg', body: 'Hi' });
      const id = nc.getAll()[0].id;
      nc.dismiss(id);
      expect(nc.count()).toBe(0);
    });

    test('dismissGroup removes all notifications for an app', () => {
      nc.add({ app: 'Messages', title: 'M1', body: '1' });
      nc.add({ app: 'Messages', title: 'M2', body: '2' });
      nc.add({ app: 'Mail', title: 'E1', body: '3' });
      nc.dismissGroup('Messages');
      expect(nc.count()).toBe(1);
      expect(nc.getAll()[0].app).toBe('Mail');
    });
  });

  describe('clearAll()', () => {
    test('removes all notifications', () => {
      nc.add({ app: 'A', title: 'x', body: 'y' });
      nc.add({ app: 'B', title: 'x', body: 'y' });
      nc.clearAll();
      expect(nc.count()).toBe(0);
    });
  });

  describe('executeAction()', () => {
    test('calls action handler when action is triggered', () => {
      const handler = jest.fn();
      nc.onAction(handler);
      nc.add({ app: 'Messages', title: 'Msg', body: 'Hi', actions: ['Reply'] });
      const id = nc.getAll()[0].id;
      nc.executeAction(id, 'Reply');
      expect(handler).toHaveBeenCalledWith(
        expect.objectContaining({ app: 'Messages', title: 'Msg' }),
        'Reply'
      );
    });

    test('auto-marks as read after action execution', () => {
      nc.onAction(() => {});
      nc.add({ app: 'Mail', title: 'E', body: 'x', actions: ['Archive'] });
      const id = nc.getAll()[0].id;
      nc.executeAction(id, 'Archive');
      expect(nc.getAll()[0].read).toBe(true);
    });
  });

  describe('Do Not Disturb', () => {
    test('when DND is on, notifications are silenced but still stored', () => {
      nc.setDoNotDisturb(true);
      const cb = jest.fn();
      nc.onNotify(cb);
      nc.add({ app: 'X', title: 'T', body: 'B' });
      expect(nc.count()).toBe(1);
      expect(cb).not.toHaveBeenCalled(); // silenced
    });

    test('when DND is off, onNotify callback fires', () => {
      nc.setDoNotDisturb(false);
      const cb = jest.fn();
      nc.onNotify(cb);
      nc.add({ app: 'X', title: 'T', body: 'B' });
      expect(cb).toHaveBeenCalledWith(expect.objectContaining({ title: 'T' }));
    });
  });
});
