/* ── Siri / Voice Assistant ────────────────────────── */
/* macOS-style voice assistant with queries, suggestions, history */

class Siri {
  constructor() {
    this.active = false;
    this.history = [];
    this.nextId = 1;
    this.maxHistory = 50;
    this.suggestions = [];
    this.handlers = {};     // command handlers by keyword
    this.preferences = {
      voice: "default",
      language: "en-US",
      alwaysListening: false,
      showSuggestions: true,
    };
  }

  /* ── Activation ───────────────── */
  activate() {
    this.active = true;
    return true;
  }

  deactivate() {
    this.active = false;
    return true;
  }

  isActive() {
    return this.active;
  }

  /* ── Query Processing ─────────── */
  query(text) {
    if (!text || !text.trim()) return null;
    text = text.trim();

    var entry = {
      id: this.nextId++,
      query: text,
      timestamp: Date.now(),
      response: null,
      type: this._classifyQuery(text),
    };

    // Try registered handlers first
    var response = this._matchHandler(text);
    if (response) {
      entry.response = response;
    } else {
      entry.response = this._defaultResponse(text, entry.type);
    }

    this.history.push(entry);
    if (this.history.length > this.maxHistory) this.history.shift();

    return entry;
  }

  _classifyQuery(text) {
    var lower = text.toLowerCase();
    if (lower.match(/^(what|who|where|when|why|how)\b/)) return "question";
    if (lower.match(/^(open|launch|start|show)\b/)) return "command";
    if (lower.match(/^(set|turn|enable|disable|change)\b/)) return "setting";
    if (lower.match(/^(remind|timer|alarm|schedule)\b/)) return "reminder";
    if (lower.match(/^(search|find|look up|google)\b/)) return "search";
    if (lower.match(/^(play|pause|skip|next|previous)\b/)) return "media";
    if (lower.match(/^(call|text|message|email|send)\b/)) return "communication";
    if (lower.match(/weather|temperature|forecast/)) return "weather";
    if (lower.match(/time|date|day|clock/)) return "time";
    if (lower.match(/calculate|math|\d+\s*[\+\-\*\/]\s*\d+/)) return "calculation";
    return "general";
  }

  _matchHandler(text) {
    var lower = text.toLowerCase();
    var keys = Object.keys(this.handlers);
    for (var i = 0; i < keys.length; i++) {
      if (lower.indexOf(keys[i].toLowerCase()) !== -1) {
        return this.handlers[keys[i]](text);
      }
    }
    return null;
  }

  _defaultResponse(text, type) {
    var responses = {
      question: { text: "Let me look that up for you.", action: "search" },
      command: { text: "I'll do that for you.", action: "execute" },
      setting: { text: "I've updated that setting.", action: "settings" },
      reminder: { text: "I've set that reminder for you.", action: "reminder" },
      search: { text: "Here's what I found.", action: "search" },
      media: { text: "Now playing.", action: "media" },
      communication: { text: "I'll send that for you.", action: "communicate" },
      weather: { text: "Here's the current weather.", action: "weather" },
      time: { text: new Date().toLocaleString(), action: "time" },
      calculation: { text: this._tryCalculate(text), action: "calculate" },
      general: { text: "Here's what I found for \"" + text + "\".", action: "search" },
    };
    return responses[type] || responses.general;
  }

  _tryCalculate(text) {
    var match = text.match(/([\d.]+)\s*([\+\-\*\/])\s*([\d.]+)/);
    if (!match) return "I couldn't calculate that.";
    var a = parseFloat(match[1]);
    var op = match[2];
    var b = parseFloat(match[3]);
    var result;
    if (op === "+") result = a + b;
    else if (op === "-") result = a - b;
    else if (op === "*") result = a * b;
    else if (op === "/") result = b !== 0 ? a / b : "undefined";
    else return "I couldn't calculate that.";
    return "The answer is " + result + ".";
  }

  /* ── Command Handlers ─────────── */
  registerHandler(keyword, fn) {
    this.handlers[keyword] = fn;
  }

  removeHandler(keyword) {
    delete this.handlers[keyword];
  }

  /* ── History ──────────────────── */
  getHistory() {
    return this.history.slice();
  }

  getLastQuery() {
    return this.history.length > 0 ? this.history[this.history.length - 1] : null;
  }

  clearHistory() {
    this.history = [];
  }

  searchHistory(query) {
    var q = query.toLowerCase();
    return this.history.filter(function (h) {
      return h.query.toLowerCase().indexOf(q) !== -1;
    });
  }

  /* ── Suggestions ──────────────── */
  setSuggestions(list) {
    this.suggestions = list.slice();
  }

  getSuggestions() {
    if (!this.preferences.showSuggestions) return [];
    return this.suggestions.slice();
  }

  addSuggestion(text) {
    if (this.suggestions.indexOf(text) === -1) {
      this.suggestions.push(text);
    }
  }

  removeSuggestion(text) {
    var idx = this.suggestions.indexOf(text);
    if (idx !== -1) this.suggestions.splice(idx, 1);
  }

  /* ── Preferences ──────────────── */
  setPreference(key, value) {
    if (key in this.preferences) {
      this.preferences[key] = value;
      return true;
    }
    return false;
  }

  getPreference(key) {
    return this.preferences[key];
  }

  getPreferences() {
    return Object.assign({}, this.preferences);
  }

  /* ── Stats ────────────────────── */
  getStats() {
    var types = {};
    this.history.forEach(function (h) {
      types[h.type] = (types[h.type] || 0) + 1;
    });
    return {
      totalQueries: this.history.length,
      byType: types,
      handlersRegistered: Object.keys(this.handlers).length,
    };
  }
}

if (typeof module !== "undefined") module.exports = { Siri: Siri };
