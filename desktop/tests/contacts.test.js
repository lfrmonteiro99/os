/**
 * TDD Tests for Contacts Application
 * RED phase: write tests before implementation
 */
const { ContactsApp } = require('../modules/contacts');

describe('ContactsApp', () => {
  let app;

  beforeEach(() => {
    app = new ContactsApp();
  });

  describe('constructor', () => {
    test('starts with empty contacts', () => {
      expect(app.listContacts()).toEqual([]);
    });

    test('starts with zero contact count', () => {
      expect(app.getContactCount()).toBe(0);
    });

    test('starts with no groups', () => {
      expect(app.listGroups()).toEqual([]);
    });
  });

  describe('addContact()', () => {
    test('adds a contact and returns an id', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      expect(typeof id).toBe('number');
      expect(id).toBeGreaterThan(0);
    });

    test('assigns unique ids', () => {
      const id1 = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const id2 = app.addContact({ firstName: 'Jane', lastName: 'Smith' });
      expect(id1).not.toBe(id2);
    });

    test('supports all contact fields', () => {
      const id = app.addContact({
        firstName: 'John',
        lastName: 'Doe',
        company: 'Acme Inc',
        email: ['john@acme.com', 'john.doe@gmail.com'],
        phone: ['+1-555-0100'],
        address: ['123 Main St, Springfield, IL'],
        birthday: '1990-06-15',
        notes: 'Met at conference',
      });
      const contact = app.getContact(id);
      expect(contact.firstName).toBe('John');
      expect(contact.lastName).toBe('Doe');
      expect(contact.company).toBe('Acme Inc');
      expect(contact.email).toEqual(['john@acme.com', 'john.doe@gmail.com']);
      expect(contact.phone).toEqual(['+1-555-0100']);
      expect(contact.address).toEqual(['123 Main St, Springfield, IL']);
      expect(contact.birthday).toBe('1990-06-15');
      expect(contact.notes).toBe('Met at conference');
    });

    test('defaults array fields to empty arrays', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const contact = app.getContact(id);
      expect(contact.email).toEqual([]);
      expect(contact.phone).toEqual([]);
      expect(contact.address).toEqual([]);
    });

    test('requires firstName', () => {
      expect(() => app.addContact({ lastName: 'Doe' })).toThrow('firstName is required');
    });

    test('requires lastName', () => {
      expect(() => app.addContact({ firstName: 'John' })).toThrow('lastName is required');
    });

    test('increments contact count', () => {
      app.addContact({ firstName: 'John', lastName: 'Doe' });
      app.addContact({ firstName: 'Jane', lastName: 'Smith' });
      expect(app.getContactCount()).toBe(2);
    });

    test('records createdAt timestamp', () => {
      const before = Date.now();
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const after = Date.now();
      const contact = app.getContact(id);
      expect(contact.createdAt).toBeGreaterThanOrEqual(before);
      expect(contact.createdAt).toBeLessThanOrEqual(after);
    });
  });

  describe('getContact()', () => {
    test('returns contact by id', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const contact = app.getContact(id);
      expect(contact.firstName).toBe('John');
      expect(contact.lastName).toBe('Doe');
    });

    test('returns null for unknown id', () => {
      expect(app.getContact(999)).toBeNull();
    });
  });

  describe('updateContact()', () => {
    test('updates contact fields', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      app.updateContact(id, { firstName: 'Jonathan', company: 'NewCo' });
      const contact = app.getContact(id);
      expect(contact.firstName).toBe('Jonathan');
      expect(contact.company).toBe('NewCo');
    });

    test('does not overwrite id or createdAt', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const original = app.getContact(id);
      app.updateContact(id, { id: 999, createdAt: 0 });
      const updated = app.getContact(id);
      expect(updated.id).toBe(id);
      expect(updated.createdAt).toBe(original.createdAt);
    });

    test('throws for unknown id', () => {
      expect(() => app.updateContact(999, { firstName: 'X' })).toThrow('Contact not found');
    });
  });

  describe('deleteContact()', () => {
    test('removes contact by id', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      app.deleteContact(id);
      expect(app.getContact(id)).toBeNull();
      expect(app.getContactCount()).toBe(0);
    });

    test('removes contact from groups and favorites on delete', () => {
      const cid = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const gid = app.createGroup('Friends');
      app.addToGroup(cid, gid);
      app.setFavorite(cid, true);
      app.deleteContact(cid);
      expect(app.getContactsByGroup(gid)).toEqual([]);
      expect(app.getFavorites()).toEqual([]);
    });
  });

  describe('listContacts()', () => {
    test('returns contacts sorted alphabetically by lastName', () => {
      app.addContact({ firstName: 'Charlie', lastName: 'Zeta' });
      app.addContact({ firstName: 'Alice', lastName: 'Alpha' });
      app.addContact({ firstName: 'Bob', lastName: 'Miller' });
      const list = app.listContacts();
      expect(list[0].lastName).toBe('Alpha');
      expect(list[1].lastName).toBe('Miller');
      expect(list[2].lastName).toBe('Zeta');
    });

    test('sorts by firstName when lastNames are equal', () => {
      app.addContact({ firstName: 'Zoe', lastName: 'Smith' });
      app.addContact({ firstName: 'Alice', lastName: 'Smith' });
      const list = app.listContacts();
      expect(list[0].firstName).toBe('Alice');
      expect(list[1].firstName).toBe('Zoe');
    });
  });

  describe('searchContacts()', () => {
    beforeEach(() => {
      app.addContact({ firstName: 'John', lastName: 'Doe', email: ['john@example.com'], phone: ['+1-555-0100'] });
      app.addContact({ firstName: 'Jane', lastName: 'Smith', email: ['jane@work.com'], phone: ['+1-555-0200'] });
      app.addContact({ firstName: 'Bob', lastName: 'Johnson', email: ['bob@test.org'], phone: ['+1-555-0300'] });
    });

    test('searches by name', () => {
      const results = app.searchContacts('john');
      expect(results.length).toBe(2); // John Doe and Bob Johnson
    });

    test('searches by email', () => {
      const results = app.searchContacts('jane@work');
      expect(results.length).toBe(1);
      expect(results[0].firstName).toBe('Jane');
    });

    test('searches by phone', () => {
      const results = app.searchContacts('0300');
      expect(results.length).toBe(1);
      expect(results[0].firstName).toBe('Bob');
    });

    test('returns empty array when no match', () => {
      expect(app.searchContacts('zzzzz')).toEqual([]);
    });
  });

  describe('groups', () => {
    test('createGroup returns group id', () => {
      const gid = app.createGroup('Friends');
      expect(typeof gid).toBe('number');
      expect(gid).toBeGreaterThan(0);
    });

    test('getGroup returns group by id', () => {
      const gid = app.createGroup('Work');
      const group = app.getGroup(gid);
      expect(group.name).toBe('Work');
      expect(group.id).toBe(gid);
    });

    test('getGroup returns null for unknown id', () => {
      expect(app.getGroup(999)).toBeNull();
    });

    test('listGroups returns all groups', () => {
      app.createGroup('Friends');
      app.createGroup('Work');
      expect(app.listGroups().length).toBe(2);
    });

    test('addToGroup adds contact to group', () => {
      const cid = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const gid = app.createGroup('Friends');
      app.addToGroup(cid, gid);
      const contacts = app.getContactsByGroup(gid);
      expect(contacts.length).toBe(1);
      expect(contacts[0].firstName).toBe('John');
    });

    test('removeFromGroup removes contact from group', () => {
      const cid = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const gid = app.createGroup('Friends');
      app.addToGroup(cid, gid);
      app.removeFromGroup(cid, gid);
      expect(app.getContactsByGroup(gid)).toEqual([]);
    });

    test('getContactsByGroup returns only contacts in group', () => {
      const c1 = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const c2 = app.addContact({ firstName: 'Jane', lastName: 'Smith' });
      const gid = app.createGroup('Work');
      app.addToGroup(c1, gid);
      const contacts = app.getContactsByGroup(gid);
      expect(contacts.length).toBe(1);
      expect(contacts[0].id).toBe(c1);
    });

    test('deleteGroup removes group but contacts remain', () => {
      const cid = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const gid = app.createGroup('Temp');
      app.addToGroup(cid, gid);
      app.deleteGroup(gid);
      expect(app.getGroup(gid)).toBeNull();
      expect(app.listGroups().length).toBe(0);
      expect(app.getContact(cid)).not.toBeNull();
    });
  });

  describe('favorites', () => {
    test('setFavorite marks a contact as favorite', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      app.setFavorite(id, true);
      expect(app.isFavorite(id)).toBe(true);
    });

    test('setFavorite can unmark a favorite', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      app.setFavorite(id, true);
      app.setFavorite(id, false);
      expect(app.isFavorite(id)).toBe(false);
    });

    test('getFavorites returns all favorited contacts', () => {
      const id1 = app.addContact({ firstName: 'John', lastName: 'Doe' });
      const id2 = app.addContact({ firstName: 'Jane', lastName: 'Smith' });
      app.addContact({ firstName: 'Bob', lastName: 'Jones' });
      app.setFavorite(id1, true);
      app.setFavorite(id2, true);
      const favs = app.getFavorites();
      expect(favs.length).toBe(2);
    });

    test('isFavorite returns false for non-favorite', () => {
      const id = app.addContact({ firstName: 'John', lastName: 'Doe' });
      expect(app.isFavorite(id)).toBe(false);
    });
  });

  describe('mergeDuplicates()', () => {
    test('merges two contacts keeping primary data and combining arrays', () => {
      const id1 = app.addContact({ firstName: 'John', lastName: 'Doe', email: ['john@work.com'], phone: ['+1-555-0100'] });
      const id2 = app.addContact({ firstName: 'John', lastName: 'Doe', email: ['john@home.com'], phone: ['+1-555-0200'] });
      const mergedId = app.mergeDuplicates(id1, id2);
      expect(mergedId).toBe(id1);
      const merged = app.getContact(mergedId);
      expect(merged.email).toContain('john@work.com');
      expect(merged.email).toContain('john@home.com');
      expect(merged.phone).toContain('+1-555-0100');
      expect(merged.phone).toContain('+1-555-0200');
      expect(app.getContact(id2)).toBeNull();
    });
  });

  describe('vCard import/export', () => {
    test('exportVCard returns vCard string', () => {
      const id = app.addContact({
        firstName: 'John',
        lastName: 'Doe',
        email: ['john@example.com'],
        phone: ['+1-555-0100'],
      });
      const vcard = app.exportVCard(id);
      expect(vcard).toContain('BEGIN:VCARD');
      expect(vcard).toContain('END:VCARD');
      expect(vcard).toContain('FN:John Doe');
      expect(vcard).toContain('N:Doe;John');
      expect(vcard).toContain('EMAIL:john@example.com');
      expect(vcard).toContain('TEL:+1-555-0100');
    });

    test('importVCard creates a contact from vCard string', () => {
      const vcard = [
        'BEGIN:VCARD',
        'VERSION:3.0',
        'N:Smith;Jane',
        'FN:Jane Smith',
        'EMAIL:jane@test.com',
        'TEL:+1-555-0999',
        'END:VCARD',
      ].join('\n');
      const id = app.importVCard(vcard);
      const contact = app.getContact(id);
      expect(contact.firstName).toBe('Jane');
      expect(contact.lastName).toBe('Smith');
      expect(contact.email).toContain('jane@test.com');
      expect(contact.phone).toContain('+1-555-0999');
    });
  });

  describe('getContactsByFirstLetter()', () => {
    test('returns contacts whose lastName starts with given letter', () => {
      app.addContact({ firstName: 'John', lastName: 'Doe' });
      app.addContact({ firstName: 'Jane', lastName: 'Davis' });
      app.addContact({ firstName: 'Bob', lastName: 'Smith' });
      const results = app.getContactsByFirstLetter('D');
      expect(results.length).toBe(2);
      expect(results.every(c => c.lastName.startsWith('D'))).toBe(true);
    });

    test('is case insensitive', () => {
      app.addContact({ firstName: 'John', lastName: 'Doe' });
      expect(app.getContactsByFirstLetter('d').length).toBe(1);
    });
  });

  describe('getRecentlyAdded()', () => {
    test('returns contacts sorted by createdAt descending', () => {
      app.addContact({ firstName: 'First', lastName: 'Added' });
      app.addContact({ firstName: 'Second', lastName: 'Added' });
      app.addContact({ firstName: 'Third', lastName: 'Added' });
      const recent = app.getRecentlyAdded(2);
      expect(recent.length).toBe(2);
      expect(recent[0].firstName).toBe('Third');
      expect(recent[1].firstName).toBe('Second');
    });
  });

  describe('getUpcomingBirthdays()', () => {
    test('returns contacts with birthdays in the next N days', () => {
      const today = new Date();
      const in5Days = new Date(today);
      in5Days.setDate(today.getDate() + 5);
      const bdayStr = (in5Days.getMonth() + 1).toString().padStart(2, '0') + '-' + in5Days.getDate().toString().padStart(2, '0');
      const fullBday = '1990-' + bdayStr;

      app.addContact({ firstName: 'Soon', lastName: 'Birthday', birthday: fullBday });
      app.addContact({ firstName: 'Far', lastName: 'Away', birthday: '1985-01-01' });

      const upcoming = app.getUpcomingBirthdays(7);
      expect(upcoming.some(c => c.firstName === 'Soon')).toBe(true);
    });
  });

  describe('validateEmail()', () => {
    test('accepts valid email', () => {
      expect(app.validateEmail('user@example.com')).toBe(true);
    });

    test('rejects email without @', () => {
      expect(app.validateEmail('userexample.com')).toBe(false);
    });

    test('rejects email without domain', () => {
      expect(app.validateEmail('user@')).toBe(false);
    });
  });

  describe('validatePhone()', () => {
    test('accepts valid phone number', () => {
      expect(app.validatePhone('+1-555-0100')).toBe(true);
    });

    test('accepts digits only', () => {
      expect(app.validatePhone('5550100')).toBe(true);
    });

    test('rejects too short phone number', () => {
      expect(app.validatePhone('12')).toBe(false);
    });

    test('rejects phone with letters', () => {
      expect(app.validatePhone('555-CALL')).toBe(false);
    });
  });
});
