/**
 * Global Undo/Redo System (Issue #65)
 * Per-window undo stacks managed by a central UndoManager.
 */

class UndoStack {
  constructor(opts) {
    opts = opts || {};
    this.maxSize = opts.maxSize || 100;
    this._undoStack = [];
    this._redoStack = [];
  }

  push(action) {
    this._undoStack.push(action);
    this._redoStack = []; // new action clears redo
    // Evict oldest if over max
    while (this._undoStack.length > this.maxSize) {
      this._undoStack.shift();
    }
  }

  undo() {
    if (this._undoStack.length === 0) return null;
    var action = this._undoStack.pop();
    this._redoStack.push(action);
    return action;
  }

  redo() {
    if (this._redoStack.length === 0) return null;
    var action = this._redoStack.pop();
    this._undoStack.push(action);
    return action;
  }

  canUndo() {
    return this._undoStack.length > 0;
  }

  canRedo() {
    return this._redoStack.length > 0;
  }

  undoSize() {
    return this._undoStack.length;
  }

  clear() {
    this._undoStack = [];
    this._redoStack = [];
  }

  peek() {
    if (this._undoStack.length === 0) return null;
    return this._undoStack[this._undoStack.length - 1];
  }
}

class UndoManager {
  constructor() {
    this._stacks = {};
  }

  _getOrCreate(windowId) {
    if (!this._stacks[windowId]) {
      this._stacks[windowId] = new UndoStack();
    }
    return this._stacks[windowId];
  }

  push(windowId, action) {
    this._getOrCreate(windowId).push(action);
  }

  undo(windowId) {
    if (!this._stacks[windowId]) return null;
    return this._stacks[windowId].undo();
  }

  redo(windowId) {
    if (!this._stacks[windowId]) return null;
    return this._stacks[windowId].redo();
  }

  canUndo(windowId) {
    if (!this._stacks[windowId]) return false;
    return this._stacks[windowId].canUndo();
  }

  canRedo(windowId) {
    if (!this._stacks[windowId]) return false;
    return this._stacks[windowId].canRedo();
  }

  removeWindow(windowId) {
    delete this._stacks[windowId];
  }

  getWindowIds() {
    return Object.keys(this._stacks);
  }
}

module.exports = { UndoStack, UndoManager };
