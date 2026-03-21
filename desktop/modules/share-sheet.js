/**
 * Share Sheet System (Issue #56)
 * Share content between apps with extensible targets.
 */

class ShareSheet {
  constructor() {
    this._targets = [
      { name: 'Messages', icon: '💬', types: ['text', 'link', 'image', 'file'] },
      { name: 'Mail', icon: '✉️', types: ['text', 'link', 'image', 'file'] },
      { name: 'Notes', icon: '📝', types: ['text', 'link', 'image'] },
      { name: 'Reminders', icon: '☑️', types: ['text', 'link'] },
      { name: 'Copy Link', icon: '🔗', types: ['text', 'link'] },
      { name: 'Add to Photos', icon: '🖼️', types: ['image'] },
      { name: 'Save as PDF', icon: '📄', types: ['text', 'link'] },
      { name: 'AirDrop', icon: '📡', types: ['text', 'link', 'image', 'file'] },
    ];
    this._visible = false;
    this._currentContent = null;
    this._shareHandler = null;
    this._history = [];
  }

  getTargets() {
    return this._targets.slice();
  }

  registerTarget(target) {
    if (!target || !target.name) {
      throw new Error('Target name is required');
    }
    var exists = this._targets.some(function (t) { return t.name === target.name; });
    if (exists) {
      throw new Error('Target already exists: ' + target.name);
    }
    this._targets.push({
      name: target.name,
      icon: target.icon || '📦',
      types: target.types || ['text'],
    });
  }

  removeTarget(name) {
    this._targets = this._targets.filter(function (t) { return t.name !== name; });
  }

  share(targetName, content) {
    var target = this._targets.find(function (t) { return t.name === targetName; });
    if (!target) {
      return { success: false, error: 'Target not found' };
    }
    if (target.types.indexOf(content.type) === -1) {
      return { success: false, error: 'Content type not supported' };
    }

    var entry = {
      target: targetName,
      content: content,
      timestamp: Date.now(),
    };
    this._history.unshift(entry);

    if (this._shareHandler) {
      this._shareHandler(target, content);
    }

    return { success: true };
  }

  getTargetsForType(type) {
    return this._targets.filter(function (t) { return t.types.indexOf(type) !== -1; });
  }

  onShare(handler) {
    this._shareHandler = handler;
  }

  open(content) {
    this._visible = true;
    this._currentContent = content;
  }

  close() {
    this._visible = false;
    this._currentContent = null;
  }

  isVisible() {
    return this._visible;
  }

  getCurrentContent() {
    return this._currentContent;
  }

  getHistory() {
    return this._history.slice();
  }

  clearHistory() {
    this._history = [];
  }
}

module.exports = { ShareSheet };
