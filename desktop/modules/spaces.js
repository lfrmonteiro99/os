/**
 * Spaces (Virtual Desktops) Feature
 * Manages multiple virtual desktops with window assignments,
 * sticky windows, wallpapers, and space lifecycle.
 */

class SpacesManager {
  constructor(options) {
    options = options || {};
    this.spaces = new Map();
    this.activeSpaceId = null;
    this.windowAssignments = new Map();
    this.stickyWindows = new Set();
    this.maxSpaces = 16;
    this.listeners = { switch: [], change: [] };
    this._nextId = 1;
    this._order = [];
    this.addSpace(options.defaultName || 'Desktop 1');
  }

  addSpace(name) {
    if (this._order.length >= this.maxSpaces) {
      throw new Error('Maximum number of spaces (' + this.maxSpaces + ') reached');
    }
    var id = this._nextId++;
    var space = { id: id, name: name, wallpaper: null };
    this.spaces.set(id, space);
    this._order.push(id);
    if (this.activeSpaceId === null) {
      this.activeSpaceId = id;
    }
    this._emitChange();
    return space;
  }

  removeSpace(id) {
    if (this._order.length <= 1) {
      throw new Error('Cannot remove the last space');
    }
    if (!this.spaces.has(id)) {
      throw new Error('Space not found: ' + id);
    }
    var idx = this._order.indexOf(id);
    var adjacentIdx = idx > 0 ? idx - 1 : 1;
    var adjacentId = this._order[adjacentIdx];

    // Redistribute windows from removed space to adjacent
    var self = this;
    this.windowAssignments.forEach(function (spaceId, windowId) {
      if (spaceId === id) {
        self.windowAssignments.set(windowId, adjacentId);
      }
    });

    this.spaces.delete(id);
    this._order.splice(idx, 1);

    if (this.activeSpaceId === id) {
      this.activeSpaceId = adjacentId;
    }
    this._emitChange();
  }

  switchTo(id) {
    if (!this.spaces.has(id)) {
      throw new Error('Space not found: ' + id);
    }
    if (this.activeSpaceId === id) {
      return;
    }
    var fromId = this.activeSpaceId;
    this.activeSpaceId = id;
    for (var i = 0; i < this.listeners.switch.length; i++) {
      this.listeners.switch[i](fromId, id);
    }
    this._emitChange();
  }

  getActiveSpace() {
    return this.spaces.get(this.activeSpaceId) || null;
  }

  getSpaces() {
    var self = this;
    return this._order.map(function (id) {
      return self.spaces.get(id);
    });
  }

  moveWindowToSpace(windowId, spaceId) {
    if (!this.spaces.has(spaceId)) {
      throw new Error('Space not found: ' + spaceId);
    }
    this.stickyWindows.delete(windowId);
    this.windowAssignments.set(windowId, spaceId);
    this._emitChange();
  }

  getWindowsInSpace(spaceId) {
    var result = [];
    var self = this;
    this.windowAssignments.forEach(function (assignedSpace, windowId) {
      if (assignedSpace === spaceId) {
        result.push(windowId);
      }
    });
    // Add sticky windows that are assigned to any space
    self.stickyWindows.forEach(function (windowId) {
      if (result.indexOf(windowId) === -1 && self.windowAssignments.has(windowId)) {
        result.push(windowId);
      }
    });
    return result;
  }

  getSpaceForWindow(windowId) {
    if (!this.windowAssignments.has(windowId)) {
      return null;
    }
    return this.windowAssignments.get(windowId);
  }

  setStickyWindow(windowId, sticky) {
    if (sticky) {
      this.stickyWindows.add(windowId);
    } else {
      this.stickyWindows.delete(windowId);
    }
    this._emitChange();
  }

  moveSpace(id, newIndex) {
    if (!this.spaces.has(id)) {
      throw new Error('Space not found: ' + id);
    }
    var oldIndex = this._order.indexOf(id);
    this._order.splice(oldIndex, 1);
    if (newIndex < 0) {
      newIndex = 0;
    }
    if (newIndex > this._order.length) {
      newIndex = this._order.length;
    }
    this._order.splice(newIndex, 0, id);
    this._emitChange();
  }

  setWallpaper(spaceId, wallpaperPath) {
    if (!this.spaces.has(spaceId)) {
      throw new Error('Space not found: ' + spaceId);
    }
    this.spaces.get(spaceId).wallpaper = wallpaperPath;
    this._emitChange();
  }

  getWallpaper(spaceId) {
    if (!this.spaces.has(spaceId)) {
      throw new Error('Space not found: ' + spaceId);
    }
    return this.spaces.get(spaceId).wallpaper;
  }

  switchToNext() {
    var idx = this._order.indexOf(this.activeSpaceId);
    var nextIdx = (idx + 1) % this._order.length;
    this.switchTo(this._order[nextIdx]);
  }

  switchToPrevious() {
    var idx = this._order.indexOf(this.activeSpaceId);
    var prevIdx = (idx - 1 + this._order.length) % this._order.length;
    this.switchTo(this._order[prevIdx]);
  }

  renameSpace(id, newName) {
    if (!this.spaces.has(id)) {
      throw new Error('Space not found: ' + id);
    }
    this.spaces.get(id).name = newName;
    this._emitChange();
  }

  onSwitch(callback) {
    this.listeners.switch.push(callback);
  }

  onChange(callback) {
    this.listeners.change.push(callback);
  }

  createFullscreenSpace(windowId, appName) {
    var space = this.addSpace(appName);
    this.moveWindowToSpace(windowId, space.id);
    this.switchTo(space.id);
    return space;
  }

  _emitChange() {
    for (var i = 0; i < this.listeners.change.length; i++) {
      this.listeners.change[i]();
    }
  }
}

module.exports = { SpacesManager };
