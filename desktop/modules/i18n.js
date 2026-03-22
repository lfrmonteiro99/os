/**
 * Multi-Language / Internationalization (Issue #49)
 * Translation key system with interpolation, fallback, and locale management.
 */

var LOCALE_NAMES = {
  en: 'English',
  pt: 'Português',
  es: 'Español',
  fr: 'Français',
  de: 'Deutsch',
  it: 'Italiano',
  ja: '日本語',
  zh: '中文',
  ko: '한국어',
  ru: 'Русский',
  ar: 'العربية',
};

class I18n {
  constructor(opts) {
    opts = opts || {};
    this._defaultLocale = opts.defaultLocale || 'en';
    this._currentLocale = this._defaultLocale;
    this._locales = {};
    this._changeCallbacks = [];
  }

  registerLocale(locale, strings) {
    this._locales[locale] = Object.assign({}, strings);
  }

  setLocale(locale) {
    if (!this._locales[locale]) {
      throw new Error('Locale not registered: ' + locale);
    }
    if (locale === this._currentLocale) return;
    var oldLocale = this._currentLocale;
    this._currentLocale = locale;
    this._changeCallbacks.forEach(function (cb) { cb(locale, oldLocale); });
  }

  getLocale() {
    return this._currentLocale;
  }

  getAvailableLocales() {
    return Object.keys(this._locales);
  }

  t(key, params) {
    var str = null;
    // Try current locale
    if (this._locales[this._currentLocale] && this._locales[this._currentLocale][key] !== undefined) {
      str = this._locales[this._currentLocale][key];
    }
    // Fallback to default locale
    if (str === null && this._currentLocale !== this._defaultLocale) {
      if (this._locales[this._defaultLocale] && this._locales[this._defaultLocale][key] !== undefined) {
        str = this._locales[this._defaultLocale][key];
      }
    }
    // Return key if not found
    if (str === null) return key;

    // Interpolation
    if (params) {
      Object.keys(params).forEach(function (k) {
        str = str.replace('{' + k + '}', String(params[k]));
      });
    }
    return str;
  }

  getLocaleStrings(locale) {
    return this._locales[locale] ? Object.assign({}, this._locales[locale]) : {};
  }

  getMissingKeys(locale) {
    if (locale === this._defaultLocale) return [];
    var defaultKeys = Object.keys(this._locales[this._defaultLocale] || {});
    var targetKeys = this._locales[locale] || {};
    return defaultKeys.filter(function (k) { return targetKeys[k] === undefined; });
  }

  onLocaleChange(callback) {
    this._changeCallbacks.push(callback);
  }

  getLocaleName(locale) {
    return LOCALE_NAMES[locale] || locale;
  }
}

module.exports = { I18n };
