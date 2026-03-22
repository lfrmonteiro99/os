/**
 * Keychain / Password Manager (Issue #58)
 * Secure credential storage with password generation.
 */

class Keychain {
  constructor() {
    this._entries = [];
    this._nextId = 1;
  }

  getAll() {
    return this._entries.slice();
  }

  count() {
    return this._entries.length;
  }

  addEntry(opts) {
    if (!opts || !opts.name) throw new Error('Entry name is required');
    var now = Date.now();
    var entry = {
      id: this._nextId++,
      name: opts.name,
      account: opts.account || '',
      password: opts.password || '',
      url: opts.url || '',
      notes: opts.notes || '',
      category: opts.category || 'password',
      createdAt: now,
      modifiedAt: now,
    };
    this._entries.push(entry);
    return entry;
  }

  getEntry(id) {
    return this._entries.find(function (e) { return e.id === id; }) || null;
  }

  updateEntry(id, updates) {
    var entry = this.getEntry(id);
    if (!entry) throw new Error('Entry not found');
    Object.keys(updates).forEach(function (key) {
      if (key !== 'id' && key !== 'createdAt') {
        entry[key] = updates[key];
      }
    });
    entry.modifiedAt = Date.now();
  }

  removeEntry(id) {
    this._entries = this._entries.filter(function (e) { return e.id !== id; });
  }

  search(query) {
    var q = query.toLowerCase();
    return this._entries.filter(function (e) {
      return e.name.toLowerCase().indexOf(q) !== -1 ||
             e.account.toLowerCase().indexOf(q) !== -1 ||
             (e.url && e.url.toLowerCase().indexOf(q) !== -1);
    });
  }

  getByCategory(category) {
    return this._entries.filter(function (e) { return e.category === category; });
  }

  getCategories() {
    return ['password', 'secure-note', 'certificate', 'key'];
  }
}

class PasswordGenerator {
  generate(opts) {
    opts = opts || {};
    var length = opts.length || 20;
    var upper = opts.uppercase !== undefined ? opts.uppercase : true;
    var lower = opts.lowercase !== undefined ? opts.lowercase : true;
    var nums = opts.numbers !== undefined ? opts.numbers : true;
    var syms = opts.symbols !== undefined ? opts.symbols : true;

    var chars = '';
    if (upper) chars += 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
    if (lower) chars += 'abcdefghijklmnopqrstuvwxyz';
    if (nums) chars += '0123456789';
    if (syms) chars += '!@#$%^&*()_+-=[]{}|;:,.<>?';

    if (chars.length === 0) throw new Error('At least one character type must be enabled');

    var result = '';
    for (var i = 0; i < length; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  evaluateStrength(password) {
    if (!password || password.length < 6) return 'weak';
    var score = 0;
    if (password.length >= 8) score++;
    if (password.length >= 12) score++;
    if (/[A-Z]/.test(password)) score++;
    if (/[a-z]/.test(password)) score++;
    if (/[0-9]/.test(password)) score++;
    if (/[^a-zA-Z0-9]/.test(password)) score++;
    if (score <= 2) return 'weak';
    if (score <= 4) return 'medium';
    return 'strong';
  }
}

module.exports = { Keychain, PasswordGenerator };
