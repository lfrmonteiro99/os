/**
 * Clipboard History Manager (Issue #13)
 * Stores last N clipboard entries with pin, search, and type support.
 */

class ClipboardHistory {
  constructor(opts) {
    opts = opts || {};
    this.maxSize = opts.maxSize || 50;
    this._entries = [];
  }

  copy(content, type, source) {
    if (!content) return;

    // Remove duplicate
    var idx = this._entries.findIndex(function (e) { return e.content === content && e.type === type; });
    if (idx !== -1) {
      this._entries.splice(idx, 1);
    }

    this._entries.unshift({
      content: content,
      type: type || 'text',
      source: source || '',
      timestamp: Date.now(),
      pinned: false,
    });

    // Enforce maxSize: keep pinned, evict oldest non-pinned
    while (this._entries.length > this.maxSize) {
      var evictIdx = -1;
      for (var i = this._entries.length - 1; i >= 0; i--) {
        if (!this._entries[i].pinned) {
          evictIdx = i;
          break;
        }
      }
      if (evictIdx === -1) break; // all pinned, can't evict
      this._entries.splice(evictIdx, 1);
    }
  }

  paste() {
    return this._entries.length > 0 ? this._entries[0].content : null;
  }

  pasteAt(index) {
    if (index < 0 || index >= this._entries.length) return null;
    return this._entries[index].content;
  }

  getAll() {
    return this._entries.slice();
  }

  size() {
    return this._entries.length;
  }

  pin(index) {
    if (index >= 0 && index < this._entries.length) {
      this._entries[index].pinned = true;
    }
  }

  unpin(index) {
    if (index >= 0 && index < this._entries.length) {
      this._entries[index].pinned = false;
    }
  }

  getPinned() {
    return this._entries.filter(function (e) { return e.pinned; });
  }

  remove(index) {
    if (index >= 0 && index < this._entries.length) {
      this._entries.splice(index, 1);
    }
  }

  clear() {
    this._entries = this._entries.filter(function (e) { return e.pinned; });
  }

  clearAll() {
    this._entries = [];
  }

  search(query) {
    var q = query.toLowerCase();
    return this._entries.filter(function (e) {
      return e.content.toLowerCase().indexOf(q) !== -1;
    });
  }
}

module.exports = { ClipboardHistory };
