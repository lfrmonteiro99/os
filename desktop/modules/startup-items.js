/**
 * Startup Items / Login Items Manager (Issue #57)
 * Manage apps that launch on login with ordering and stagger delays.
 */

var VALID_TYPES = ['app', 'background', 'agent'];

class StartupManager {
  constructor() {
    this._items = [];
    this._nextId = 1;
  }

  getAll() {
    return this._items.slice();
  }

  count() {
    return this._items.length;
  }

  getItem(id) {
    return this._items.find(function (i) { return i.id === id; }) || null;
  }

  addItem(opts) {
    if (!opts || !opts.name) throw new Error('Item name is required');
    if (VALID_TYPES.indexOf(opts.type) === -1) throw new Error('Invalid item type: ' + opts.type);
    if (this._items.some(function (i) { return i.name === opts.name; })) {
      throw new Error('Item already exists: ' + opts.name);
    }
    var item = {
      id: this._nextId++,
      name: opts.name,
      type: opts.type,
      enabled: opts.enabled !== undefined ? opts.enabled : true,
      hidden: opts.hidden || false,
      createdAt: Date.now(),
    };
    this._items.push(item);
    return item;
  }

  removeItem(id) {
    this._items = this._items.filter(function (i) { return i.id !== id; });
  }

  toggleItem(id) {
    var item = this.getItem(id);
    if (item) item.enabled = !item.enabled;
  }

  updateItem(id, updates) {
    var item = this.getItem(id);
    if (!item) throw new Error('Item not found');
    Object.keys(updates).forEach(function (key) {
      if (key !== 'id' && key !== 'createdAt') {
        item[key] = updates[key];
      }
    });
  }

  getByType(type) {
    return this._items.filter(function (i) { return i.type === type; });
  }

  getEnabled() {
    return this._items.filter(function (i) { return i.enabled; });
  }

  reorder(id, newIndex) {
    var idx = this._items.findIndex(function (i) { return i.id === id; });
    if (idx === -1) return;
    var item = this._items.splice(idx, 1)[0];
    this._items.splice(newIndex, 0, item);
  }

  getLaunchOrder(stagger) {
    stagger = stagger || 500;
    return this.getEnabled().map(function (item, i) {
      return { name: item.name, type: item.type, delay: i * stagger, hidden: item.hidden };
    });
  }

  export() {
    return JSON.stringify(this._items.map(function (i) {
      return { name: i.name, type: i.type, enabled: i.enabled, hidden: i.hidden };
    }));
  }

  import(json) {
    var self = this;
    var items = JSON.parse(json);
    items.forEach(function (i) {
      if (!self._items.some(function (existing) { return existing.name === i.name; })) {
        self.addItem(i);
      }
    });
  }
}

module.exports = { StartupManager };
