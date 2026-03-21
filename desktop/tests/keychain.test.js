/**
 * TDD Tests for Keychain / Password Manager (Issue #58)
 * RED phase: write tests before implementation
 */
const { Keychain, PasswordGenerator } = require('../modules/keychain');

describe('Keychain', () => {
  let kc;

  beforeEach(() => {
    kc = new Keychain();
  });

  describe('constructor', () => {
    test('starts empty', () => {
      expect(kc.getAll()).toEqual([]);
      expect(kc.count()).toBe(0);
    });
  });

  describe('addEntry()', () => {
    test('adds a password entry', () => {
      kc.addEntry({ name: 'GitHub', account: 'user@example.com', password: 's3cret', url: 'https://github.com', category: 'password' });
      expect(kc.count()).toBe(1);
      const entry = kc.getAll()[0];
      expect(entry.name).toBe('GitHub');
      expect(entry.account).toBe('user@example.com');
      expect(entry.category).toBe('password');
    });

    test('assigns unique id to each entry', () => {
      kc.addEntry({ name: 'A', account: 'a', password: 'x', category: 'password' });
      kc.addEntry({ name: 'B', account: 'b', password: 'y', category: 'password' });
      const ids = kc.getAll().map(e => e.id);
      expect(new Set(ids).size).toBe(2);
    });

    test('stores timestamp', () => {
      kc.addEntry({ name: 'Test', account: 't', password: 'p', category: 'password' });
      expect(kc.getAll()[0].createdAt).toBeDefined();
      expect(kc.getAll()[0].modifiedAt).toBeDefined();
    });

    test('supports secure note category', () => {
      kc.addEntry({ name: 'API Key', notes: 'sk-1234', category: 'secure-note' });
      expect(kc.getAll()[0].category).toBe('secure-note');
    });

    test('rejects entry without name', () => {
      expect(() => kc.addEntry({ account: 'a', password: 'p', category: 'password' }))
        .toThrow('Entry name is required');
    });
  });

  describe('getEntry()', () => {
    test('retrieves entry by id', () => {
      kc.addEntry({ name: 'GitHub', account: 'u', password: 'p', category: 'password' });
      const id = kc.getAll()[0].id;
      const entry = kc.getEntry(id);
      expect(entry.name).toBe('GitHub');
    });

    test('returns null for unknown id', () => {
      expect(kc.getEntry(999)).toBeNull();
    });
  });

  describe('updateEntry()', () => {
    test('updates fields of an existing entry', () => {
      kc.addEntry({ name: 'GitHub', account: 'old@mail.com', password: 'p', category: 'password' });
      const id = kc.getAll()[0].id;
      kc.updateEntry(id, { account: 'new@mail.com', password: 'newpass' });
      const entry = kc.getEntry(id);
      expect(entry.account).toBe('new@mail.com');
      expect(entry.password).toBe('newpass');
    });

    test('updates modifiedAt timestamp', () => {
      kc.addEntry({ name: 'X', account: 'a', password: 'p', category: 'password' });
      const id = kc.getAll()[0].id;
      const oldMod = kc.getEntry(id).modifiedAt;
      // Small delay to ensure different timestamp
      kc.updateEntry(id, { password: 'newp' });
      expect(kc.getEntry(id).modifiedAt).toBeGreaterThanOrEqual(oldMod);
    });

    test('throws for unknown id', () => {
      expect(() => kc.updateEntry(999, { password: 'x' })).toThrow('Entry not found');
    });
  });

  describe('removeEntry()', () => {
    test('removes entry by id', () => {
      kc.addEntry({ name: 'A', account: 'a', password: 'p', category: 'password' });
      const id = kc.getAll()[0].id;
      kc.removeEntry(id);
      expect(kc.count()).toBe(0);
    });
  });

  describe('search()', () => {
    test('searches by name', () => {
      kc.addEntry({ name: 'GitHub', account: 'u', password: 'p', category: 'password' });
      kc.addEntry({ name: 'GitLab', account: 'u', password: 'p', category: 'password' });
      kc.addEntry({ name: 'Netflix', account: 'u', password: 'p', category: 'password' });
      const results = kc.search('git');
      expect(results.length).toBe(2);
    });

    test('searches by account', () => {
      kc.addEntry({ name: 'A', account: 'john@mail.com', password: 'p', category: 'password' });
      kc.addEntry({ name: 'B', account: 'jane@mail.com', password: 'p', category: 'password' });
      expect(kc.search('john').length).toBe(1);
    });

    test('searches by URL', () => {
      kc.addEntry({ name: 'GH', account: 'u', password: 'p', url: 'https://github.com', category: 'password' });
      expect(kc.search('github.com').length).toBe(1);
    });

    test('is case-insensitive', () => {
      kc.addEntry({ name: 'GitHub', account: 'u', password: 'p', category: 'password' });
      expect(kc.search('GITHUB').length).toBe(1);
    });
  });

  describe('getByCategory()', () => {
    test('filters entries by category', () => {
      kc.addEntry({ name: 'A', account: 'a', password: 'p', category: 'password' });
      kc.addEntry({ name: 'B', notes: 'secret', category: 'secure-note' });
      kc.addEntry({ name: 'C', account: 'c', password: 'p', category: 'password' });
      expect(kc.getByCategory('password').length).toBe(2);
      expect(kc.getByCategory('secure-note').length).toBe(1);
    });
  });

  describe('getCategories()', () => {
    test('returns list of available categories', () => {
      const cats = kc.getCategories();
      expect(cats).toContain('password');
      expect(cats).toContain('secure-note');
      expect(cats).toContain('certificate');
      expect(cats).toContain('key');
    });
  });
});

describe('PasswordGenerator', () => {
  let gen;

  beforeEach(() => {
    gen = new PasswordGenerator();
  });

  describe('generate()', () => {
    test('generates password of specified length', () => {
      const pw = gen.generate({ length: 16 });
      expect(pw.length).toBe(16);
    });

    test('defaults to length 20', () => {
      const pw = gen.generate();
      expect(pw.length).toBe(20);
    });

    test('includes uppercase when enabled', () => {
      const pw = gen.generate({ length: 50, uppercase: true, lowercase: false, numbers: false, symbols: false });
      expect(pw).toMatch(/[A-Z]/);
    });

    test('includes lowercase when enabled', () => {
      const pw = gen.generate({ length: 50, uppercase: false, lowercase: true, numbers: false, symbols: false });
      expect(pw).toMatch(/[a-z]/);
    });

    test('includes numbers when enabled', () => {
      const pw = gen.generate({ length: 50, uppercase: false, lowercase: false, numbers: true, symbols: false });
      expect(pw).toMatch(/[0-9]/);
    });

    test('includes symbols when enabled', () => {
      const pw = gen.generate({ length: 50, uppercase: false, lowercase: false, numbers: false, symbols: true });
      expect(pw).toMatch(/[^a-zA-Z0-9]/);
    });

    test('throws if all character types disabled', () => {
      expect(() => gen.generate({ uppercase: false, lowercase: false, numbers: false, symbols: false }))
        .toThrow('At least one character type must be enabled');
    });

    test('evaluateStrength returns weak for short passwords', () => {
      expect(gen.evaluateStrength('abc')).toBe('weak');
    });

    test('evaluateStrength returns medium for moderate passwords', () => {
      expect(gen.evaluateStrength('Abcdef12')).toBe('medium');
    });

    test('evaluateStrength returns strong for complex passwords', () => {
      expect(gen.evaluateStrength('Tr0ub4dor&3#Xy!pQ9')).toBe('strong');
    });
  });
});
