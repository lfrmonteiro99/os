/**
 * Contacts Application
 * Contact management with groups, favorites, search, vCard support, and more.
 */

class ContactsApp {
  constructor() {
    this.contacts = new Map();
    this.groups = new Map();
    this.favorites = new Set();
    this._nextContactId = 1;
    this._nextGroupId = 1;
    this._groupMembers = new Map(); // groupId -> Set of contactIds
    this._addOrder = 0;
  }

  // --- Contacts ---

  addContact(opts) {
    if (!opts || !opts.firstName) throw new Error('firstName is required');
    if (!opts.lastName) throw new Error('lastName is required');
    var id = this._nextContactId++;
    var contact = {
      id: id,
      firstName: opts.firstName,
      lastName: opts.lastName,
      company: opts.company || '',
      email: opts.email || [],
      phone: opts.phone || [],
      address: opts.address || [],
      birthday: opts.birthday || null,
      notes: opts.notes || '',
      createdAt: Date.now(),
      _addOrder: this._addOrder++,
    };
    this.contacts.set(id, contact);
    return id;
  }

  getContact(id) {
    return this.contacts.get(id) || null;
  }

  updateContact(id, updates) {
    var contact = this.contacts.get(id);
    if (!contact) throw new Error('Contact not found');
    Object.keys(updates).forEach(function (key) {
      if (key !== 'id' && key !== 'createdAt') {
        contact[key] = updates[key];
      }
    });
  }

  deleteContact(id) {
    this.contacts.delete(id);
    this.favorites.delete(id);
    var self = this;
    this._groupMembers.forEach(function (members) {
      members.delete(id);
    });
  }

  listContacts() {
    var arr = Array.from(this.contacts.values());
    arr.sort(function (a, b) {
      var cmp = a.lastName.localeCompare(b.lastName);
      if (cmp !== 0) return cmp;
      return a.firstName.localeCompare(b.firstName);
    });
    return arr;
  }

  getContactCount() {
    return this.contacts.size;
  }

  searchContacts(query) {
    var q = query.toLowerCase();
    return Array.from(this.contacts.values()).filter(function (c) {
      var fullName = (c.firstName + ' ' + c.lastName).toLowerCase();
      if (fullName.indexOf(q) !== -1) return true;
      if (c.email.some(function (e) { return e.toLowerCase().indexOf(q) !== -1; })) return true;
      if (c.phone.some(function (p) { return p.toLowerCase().indexOf(q) !== -1; })) return true;
      return false;
    });
  }

  getContactsByFirstLetter(letter) {
    var l = letter.toUpperCase();
    return Array.from(this.contacts.values()).filter(function (c) {
      return c.lastName.toUpperCase().charAt(0) === l;
    });
  }

  getRecentlyAdded(limit) {
    var arr = Array.from(this.contacts.values());
    arr.sort(function (a, b) {
      var diff = b.createdAt - a.createdAt;
      if (diff !== 0) return diff;
      return b._addOrder - a._addOrder;
    });
    return arr.slice(0, limit);
  }

  // --- Groups ---

  createGroup(name) {
    var id = this._nextGroupId++;
    this.groups.set(id, { id: id, name: name });
    this._groupMembers.set(id, new Set());
    return id;
  }

  getGroup(id) {
    return this.groups.get(id) || null;
  }

  listGroups() {
    return Array.from(this.groups.values());
  }

  deleteGroup(id) {
    this.groups.delete(id);
    this._groupMembers.delete(id);
  }

  addToGroup(contactId, groupId) {
    var members = this._groupMembers.get(groupId);
    if (members) {
      members.add(contactId);
    }
  }

  removeFromGroup(contactId, groupId) {
    var members = this._groupMembers.get(groupId);
    if (members) {
      members.delete(contactId);
    }
  }

  getContactsByGroup(groupId) {
    var members = this._groupMembers.get(groupId);
    if (!members) return [];
    var self = this;
    var result = [];
    members.forEach(function (cid) {
      var contact = self.contacts.get(cid);
      if (contact) result.push(contact);
    });
    return result;
  }

  // --- Favorites ---

  setFavorite(contactId, isFav) {
    if (isFav) {
      this.favorites.add(contactId);
    } else {
      this.favorites.delete(contactId);
    }
  }

  isFavorite(contactId) {
    return this.favorites.has(contactId);
  }

  getFavorites() {
    var self = this;
    var result = [];
    this.favorites.forEach(function (cid) {
      var contact = self.contacts.get(cid);
      if (contact) result.push(contact);
    });
    return result;
  }

  // --- Merge ---

  mergeDuplicates(primaryId, secondaryId) {
    var primary = this.contacts.get(primaryId);
    var secondary = this.contacts.get(secondaryId);
    if (!primary || !secondary) throw new Error('Contact not found');

    ['email', 'phone', 'address'].forEach(function (field) {
      secondary[field].forEach(function (val) {
        if (primary[field].indexOf(val) === -1) {
          primary[field].push(val);
        }
      });
    });

    if (!primary.company && secondary.company) {
      primary.company = secondary.company;
    }
    if (!primary.birthday && secondary.birthday) {
      primary.birthday = secondary.birthday;
    }
    if (!primary.notes && secondary.notes) {
      primary.notes = secondary.notes;
    }

    this.deleteContact(secondaryId);
    return primaryId;
  }

  // --- vCard ---

  exportVCard(contactId) {
    var c = this.contacts.get(contactId);
    if (!c) throw new Error('Contact not found');
    var lines = [
      'BEGIN:VCARD',
      'VERSION:3.0',
      'N:' + c.lastName + ';' + c.firstName,
      'FN:' + c.firstName + ' ' + c.lastName,
    ];
    c.email.forEach(function (e) { lines.push('EMAIL:' + e); });
    c.phone.forEach(function (p) { lines.push('TEL:' + p); });
    if (c.company) lines.push('ORG:' + c.company);
    if (c.birthday) lines.push('BDAY:' + c.birthday);
    lines.push('END:VCARD');
    return lines.join('\n');
  }

  importVCard(vcardStr) {
    var lines = vcardStr.split('\n');
    var data = { firstName: '', lastName: '', email: [], phone: [] };
    lines.forEach(function (line) {
      var trimmed = line.trim();
      if (trimmed.indexOf('N:') === 0) {
        var parts = trimmed.substring(2).split(';');
        data.lastName = parts[0] || '';
        data.firstName = parts[1] || '';
      } else if (trimmed.indexOf('EMAIL:') === 0) {
        data.email.push(trimmed.substring(6));
      } else if (trimmed.indexOf('TEL:') === 0) {
        data.phone.push(trimmed.substring(4));
      } else if (trimmed.indexOf('ORG:') === 0) {
        data.company = trimmed.substring(4);
      } else if (trimmed.indexOf('BDAY:') === 0) {
        data.birthday = trimmed.substring(5);
      }
    });
    return this.addContact(data);
  }

  // --- Birthdays ---

  getUpcomingBirthdays(days) {
    var today = new Date();
    today.setHours(0, 0, 0, 0);
    var results = [];
    this.contacts.forEach(function (c) {
      if (!c.birthday) return;
      var bday = new Date(c.birthday);
      var thisYearBday = new Date(today.getFullYear(), bday.getMonth(), bday.getDate());
      if (thisYearBday < today) {
        thisYearBday.setFullYear(thisYearBday.getFullYear() + 1);
      }
      var diffMs = thisYearBday.getTime() - today.getTime();
      var diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));
      if (diffDays >= 0 && diffDays <= days) {
        results.push(c);
      }
    });
    return results;
  }

  // --- Validation ---

  validateEmail(email) {
    var re = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return re.test(email);
  }

  validatePhone(phone) {
    var digits = phone.replace(/[\s\-\+\(\)\.]/g, '');
    if (digits.length < 3) return false;
    return /^\d+$/.test(digits);
  }
}

module.exports = { ContactsApp };
