/**
 * System Preferences Manager (macOS-style preference panes)
 * Manage system settings across multiple preference panes.
 */

var VALID_DOCK_POSITIONS = ['bottom', 'left', 'right'];
var VALID_APPEARANCES = ['light', 'dark', 'auto'];
var VALID_SIDEBAR_SIZES = ['small', 'medium', 'large'];
var VALID_SCROLL_BEHAVIORS = ['automatic', 'always', 'whenScrolling'];
var VALID_DOCK_ANIMATIONS = ['genie', 'scale', 'suck'];

function deepCopy(obj) {
  return JSON.parse(JSON.stringify(obj));
}

function getDefaultPanes() {
  return {
    General: {
      settings: {
        appearance: 'light',
        accentColor: 'blue',
        sidebarIconSize: 'medium',
        scrollBarBehavior: 'automatic'
      },
      keywords: ['appearance', 'accent', 'color', 'sidebar', 'icon', 'scroll', 'theme', 'dark', 'light']
    },
    Desktop: {
      settings: {
        wallpaperPath: '/System/Library/Desktop Pictures/default.jpg',
        screenSaverName: 'Flurry',
        screenSaverTimeout: 300
      },
      keywords: ['wallpaper', 'screen saver', 'background', 'desktop picture', 'timeout']
    },
    Dock: {
      settings: {
        size: 48,
        magnification: false,
        position: 'bottom',
        autoHide: false,
        animation: 'genie'
      },
      keywords: ['dock', 'size', 'magnification', 'position', 'auto-hide', 'animation', 'genie', 'minimize']
    },
    Display: {
      settings: {
        resolution: '2560x1600',
        brightness: 75,
        nightShift: false,
        trueTone: true
      },
      keywords: ['display', 'resolution', 'brightness', 'night shift', 'true tone', 'monitor', 'screen']
    },
    Sound: {
      settings: {
        outputVolume: 50,
        inputVolume: 50,
        outputDevice: 'Internal Speakers',
        inputDevice: 'Internal Microphone',
        alertSound: 'Tink'
      },
      keywords: ['sound', 'volume', 'output', 'input', 'speaker', 'microphone', 'alert', 'audio']
    },
    Keyboard: {
      settings: {
        keyRepeatRate: 6,
        delayUntilRepeat: 2,
        shortcuts: {
          copy: 'Cmd+C',
          paste: 'Cmd+V',
          cut: 'Cmd+X',
          undo: 'Cmd+Z',
          selectAll: 'Cmd+A'
        }
      },
      keywords: ['keyboard', 'key repeat', 'delay', 'shortcuts', 'typing', 'input']
    },
    Mouse: {
      settings: {
        trackingSpeed: 5,
        scrollDirection: 'natural',
        secondaryClick: true
      },
      keywords: ['mouse', 'tracking', 'scroll', 'click', 'pointer', 'cursor']
    },
    Network: {
      settings: {
        wifi: true,
        networkName: '',
        dns: '8.8.8.8',
        proxy: 'none'
      },
      keywords: ['network', 'wifi', 'ethernet', 'dns', 'proxy', 'internet', 'connection']
    },
    Users: {
      settings: {
        autoLogin: false,
        guestAccount: false,
        loginWindowShow: 'list'
      },
      keywords: ['users', 'accounts', 'login', 'guest', 'password', 'groups']
    },
    Security: {
      settings: {
        requirePassword: true,
        passwordDelay: 5,
        firewall: true,
        fileVault: false
      },
      keywords: ['security', 'privacy', 'password', 'firewall', 'filevault', 'encryption', 'lock']
    }
  };
}

var VALIDATORS = {
  Sound: {
    outputVolume: function (v) { return typeof v === 'number' && v >= 0 && v <= 100; },
    inputVolume: function (v) { return typeof v === 'number' && v >= 0 && v <= 100; }
  },
  Dock: {
    position: function (v) { return VALID_DOCK_POSITIONS.indexOf(v) !== -1; }
  },
  Display: {
    brightness: function (v) { return typeof v === 'number' && v >= 0 && v <= 100; }
  },
  General: {
    appearance: function (v) { return VALID_APPEARANCES.indexOf(v) !== -1; },
    sidebarIconSize: function (v) { return VALID_SIDEBAR_SIZES.indexOf(v) !== -1; },
    scrollBarBehavior: function (v) { return VALID_SCROLL_BEHAVIORS.indexOf(v) !== -1; }
  }
};

class SystemPreferences {
  constructor(options) {
    this.currentPane = null;
    this.listeners = new Map();
    this.panes = new Map();
    this._defaults = {};
    this._initDefaultPanes();
  }

  _initDefaultPanes() {
    var defaults = getDefaultPanes();
    this._defaults = defaults;
    var self = this;
    Object.keys(defaults).forEach(function (name) {
      self.panes.set(name, {
        settings: deepCopy(defaults[name].settings),
        keywords: defaults[name].keywords.slice()
      });
    });
  }

  listPanes() {
    var names = [];
    this.panes.forEach(function (value, key) {
      names.push(key);
    });
    return names;
  }

  openPane(name) {
    if (!this.panes.has(name)) throw new Error('Unknown pane: ' + name);
    this.currentPane = name;
  }

  closePane() {
    this.currentPane = null;
  }

  getCurrentPane() {
    return this.currentPane;
  }

  searchPanes(keyword) {
    var lower = keyword.toLowerCase();
    var results = [];
    this.panes.forEach(function (pane, name) {
      var found = pane.keywords.some(function (kw) {
        return kw.toLowerCase().indexOf(lower) !== -1;
      });
      if (found) {
        results.push(name);
      }
    });
    return results;
  }

  getSetting(paneName, key) {
    if (!this.panes.has(paneName)) throw new Error('Unknown pane: ' + paneName);
    var pane = this.panes.get(paneName);
    if (!(key in pane.settings)) throw new Error('Unknown setting: ' + key);
    var val = pane.settings[key];
    if (val !== null && typeof val === 'object' && !Array.isArray(val)) {
      return deepCopy(val);
    }
    return val;
  }

  setSetting(paneName, key, value) {
    if (!this.panes.has(paneName)) throw new Error('Unknown pane: ' + paneName);
    var pane = this.panes.get(paneName);
    if (!(key in pane.settings)) throw new Error('Unknown setting: ' + key);
    if (VALIDATORS[paneName] && VALIDATORS[paneName][key]) {
      if (!VALIDATORS[paneName][key](value)) {
        throw new Error('Invalid value for ' + paneName + '.' + key + ': ' + value);
      }
    }
    pane.settings[key] = value;
    this._notifyListeners(paneName, key, value);
  }

  resetPane(paneName) {
    if (!this.panes.has(paneName)) throw new Error('Unknown pane: ' + paneName);
    if (!this._defaults[paneName]) throw new Error('No defaults for pane: ' + paneName);
    var pane = this.panes.get(paneName);
    pane.settings = deepCopy(this._defaults[paneName].settings);
  }

  exportSettings() {
    var result = {};
    this.panes.forEach(function (pane, name) {
      result[name] = deepCopy(pane.settings);
    });
    return JSON.stringify(result);
  }

  importSettings(json) {
    var data = JSON.parse(json);
    var self = this;
    Object.keys(data).forEach(function (paneName) {
      if (self.panes.has(paneName)) {
        var pane = self.panes.get(paneName);
        Object.keys(data[paneName]).forEach(function (key) {
          if (key in pane.settings) {
            pane.settings[key] = data[paneName][key];
          }
        });
      }
    });
  }

  onChange(paneName, callback) {
    if (!this.listeners.has(paneName)) {
      this.listeners.set(paneName, []);
    }
    this.listeners.get(paneName).push(callback);
  }

  _notifyListeners(paneName, key, value) {
    if (this.listeners.has(paneName)) {
      this.listeners.get(paneName).forEach(function (cb) {
        cb(paneName, key, value);
      });
    }
  }
}

module.exports = { SystemPreferences };
