/**
 * Enhanced Notification Center (Issue #52)
 * Grouping by app, actions, DND support.
 */

class NotificationCenter {
  constructor() {
    this._notifications = [];
    this._nextId = 1;
    this._dnd = false;
    this._actionHandler = null;
    this._notifyHandler = null;
  }

  add(opts) {
    var notification = {
      id: this._nextId++,
      app: opts.app,
      title: opts.title,
      body: opts.body,
      icon: opts.icon || '',
      actions: opts.actions || [],
      read: false,
      timestamp: Date.now(),
    };
    this._notifications.unshift(notification);

    if (!this._dnd && this._notifyHandler) {
      this._notifyHandler(notification);
    }

    return notification;
  }

  getAll() {
    return this._notifications.slice();
  }

  count() {
    return this._notifications.length;
  }

  getGrouped() {
    var groups = {};
    this._notifications.forEach(function (n) {
      if (!groups[n.app]) groups[n.app] = [];
      groups[n.app].push(n);
    });
    return groups;
  }

  getGroupSummary() {
    var groups = this.getGrouped();
    return Object.keys(groups).map(function (app) {
      var items = groups[app];
      return {
        app: app,
        count: items.length,
        latest: items[0].title,
      };
    });
  }

  markRead(id) {
    var n = this._findById(id);
    if (n) n.read = true;
  }

  markAllRead() {
    this._notifications.forEach(function (n) { n.read = true; });
  }

  getUnreadCount() {
    return this._notifications.filter(function (n) { return !n.read; }).length;
  }

  dismiss(id) {
    this._notifications = this._notifications.filter(function (n) { return n.id !== id; });
  }

  dismissGroup(app) {
    this._notifications = this._notifications.filter(function (n) { return n.app !== app; });
  }

  clearAll() {
    this._notifications = [];
  }

  executeAction(id, actionName) {
    var n = this._findById(id);
    if (n && this._actionHandler) {
      this._actionHandler(n, actionName);
      n.read = true;
    }
  }

  onAction(handler) {
    this._actionHandler = handler;
  }

  onNotify(handler) {
    this._notifyHandler = handler;
  }

  setDoNotDisturb(enabled) {
    this._dnd = !!enabled;
  }

  _findById(id) {
    for (var i = 0; i < this._notifications.length; i++) {
      if (this._notifications[i].id === id) return this._notifications[i];
    }
    return null;
  }
}

module.exports = { NotificationCenter };
