/**
 * TDD Tests for Multi-Language / Internationalization (Issue #49)
 * RED phase: write tests before implementation
 */
const { I18n } = require('../modules/i18n');

describe('I18n', () => {
  let i18n;

  const enStrings = {
    'menu.file': 'File',
    'menu.edit': 'Edit',
    'menu.view': 'View',
    'menu.go': 'Go',
    'menu.window': 'Window',
    'menu.help': 'Help',
    'dock.finder': 'Finder',
    'dock.safari': 'Safari',
    'dock.messages': 'Messages',
    'settings.general': 'General',
    'settings.appearance': 'Appearance',
    'app.notes': 'Notes',
    'app.calculator': 'Calculator',
    'common.ok': 'OK',
    'common.cancel': 'Cancel',
    'common.save': 'Save',
    'common.delete': 'Delete',
    'common.close': 'Close',
    'greeting': 'Hello, {name}!',
    'items_count': '{count} items',
  };

  const ptStrings = {
    'menu.file': 'Ficheiro',
    'menu.edit': 'Editar',
    'menu.view': 'Visualização',
    'menu.go': 'Ir',
    'menu.window': 'Janela',
    'menu.help': 'Ajuda',
    'dock.finder': 'Finder',
    'dock.safari': 'Safari',
    'dock.messages': 'Mensagens',
    'settings.general': 'Geral',
    'settings.appearance': 'Aparência',
    'app.notes': 'Notas',
    'app.calculator': 'Calculadora',
    'common.ok': 'OK',
    'common.cancel': 'Cancelar',
    'common.save': 'Guardar',
    'common.delete': 'Eliminar',
    'common.close': 'Fechar',
    'greeting': 'Olá, {name}!',
    'items_count': '{count} itens',
  };

  const esStrings = {
    'menu.file': 'Archivo',
    'menu.edit': 'Editar',
    'common.ok': 'Aceptar',
    'common.cancel': 'Cancelar',
    'greeting': '¡Hola, {name}!',
  };

  beforeEach(() => {
    i18n = new I18n({ defaultLocale: 'en' });
    i18n.registerLocale('en', enStrings);
    i18n.registerLocale('pt', ptStrings);
    i18n.registerLocale('es', esStrings);
  });

  describe('constructor', () => {
    test('sets default locale', () => {
      expect(i18n.getLocale()).toBe('en');
    });
  });

  describe('registerLocale()', () => {
    test('registers a new locale', () => {
      i18n.registerLocale('fr', { 'common.ok': 'D\'accord' });
      expect(i18n.getAvailableLocales()).toContain('fr');
    });
  });

  describe('setLocale()', () => {
    test('changes current locale', () => {
      i18n.setLocale('pt');
      expect(i18n.getLocale()).toBe('pt');
    });

    test('throws for unregistered locale', () => {
      expect(() => i18n.setLocale('ja')).toThrow('Locale not registered: ja');
    });
  });

  describe('t() - basic translation', () => {
    test('returns translation for current locale', () => {
      expect(i18n.t('menu.file')).toBe('File');
    });

    test('returns Portuguese translation after locale change', () => {
      i18n.setLocale('pt');
      expect(i18n.t('menu.file')).toBe('Ficheiro');
      expect(i18n.t('common.save')).toBe('Guardar');
    });

    test('returns Spanish translation', () => {
      i18n.setLocale('es');
      expect(i18n.t('menu.file')).toBe('Archivo');
    });

    test('falls back to default locale for missing key', () => {
      i18n.setLocale('es');
      // 'dock.finder' not in es, should fall back to en
      expect(i18n.t('dock.finder')).toBe('Finder');
    });

    test('returns key itself when not found in any locale', () => {
      expect(i18n.t('nonexistent.key')).toBe('nonexistent.key');
    });
  });

  describe('t() - interpolation', () => {
    test('replaces {placeholders} with values', () => {
      expect(i18n.t('greeting', { name: 'World' })).toBe('Hello, World!');
    });

    test('interpolation works in other locales', () => {
      i18n.setLocale('pt');
      expect(i18n.t('greeting', { name: 'Mundo' })).toBe('Olá, Mundo!');
    });

    test('replaces multiple placeholders', () => {
      expect(i18n.t('items_count', { count: 42 })).toBe('42 items');
    });

    test('leaves placeholder if value not provided', () => {
      expect(i18n.t('greeting', {})).toBe('Hello, {name}!');
    });
  });

  describe('getAvailableLocales()', () => {
    test('returns all registered locales', () => {
      const locales = i18n.getAvailableLocales();
      expect(locales).toContain('en');
      expect(locales).toContain('pt');
      expect(locales).toContain('es');
      expect(locales.length).toBe(3);
    });
  });

  describe('getLocaleStrings()', () => {
    test('returns all strings for a locale', () => {
      const strings = i18n.getLocaleStrings('en');
      expect(strings['menu.file']).toBe('File');
    });

    test('returns empty object for unknown locale', () => {
      expect(i18n.getLocaleStrings('unknown')).toEqual({});
    });
  });

  describe('getMissingKeys()', () => {
    test('finds keys present in default but missing in target', () => {
      const missing = i18n.getMissingKeys('es');
      expect(missing).toContain('dock.finder');
      expect(missing).toContain('settings.general');
      expect(missing).not.toContain('menu.file'); // present in es
    });

    test('returns empty for default locale', () => {
      expect(i18n.getMissingKeys('en')).toEqual([]);
    });
  });

  describe('onLocaleChange()', () => {
    test('calls callback when locale changes', () => {
      const cb = jest.fn();
      i18n.onLocaleChange(cb);
      i18n.setLocale('pt');
      expect(cb).toHaveBeenCalledWith('pt', 'en');
    });

    test('does not call callback if same locale set', () => {
      const cb = jest.fn();
      i18n.onLocaleChange(cb);
      i18n.setLocale('en');
      expect(cb).not.toHaveBeenCalled();
    });
  });

  describe('getLocaleName()', () => {
    test('returns human-readable locale name', () => {
      expect(i18n.getLocaleName('en')).toBe('English');
      expect(i18n.getLocaleName('pt')).toBe('Português');
      expect(i18n.getLocaleName('es')).toBe('Español');
    });

    test('returns locale code for unknown locale', () => {
      expect(i18n.getLocaleName('xx')).toBe('xx');
    });
  });
});
