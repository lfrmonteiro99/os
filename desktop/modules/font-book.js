/* ── Font Book ────────────────────────────────────── */
/* macOS-style font management, collections, preview */

class FontBook {
  constructor() {
    this.fonts = [];
    this.nextId = 1;
    this.collections = [
      { id: 1, name: "All Fonts", system: true, fontIds: [] },
      { id: 2, name: "Recently Added", system: true, fontIds: [] },
    ];
    this.nextCollId = 3;
    this.previewText = "The quick brown fox jumps over the lazy dog";
    this.previewSize = 24;
    this.favorites = [];
    this.disabled = [];
  }

  /* ── Font Management ──────────── */
  addFont(opts) {
    var existing = this.fonts.find(function (f) { return f.family === opts.family && f.style === (opts.style || "Regular"); });
    if (existing) return existing;
    var font = {
      id: this.nextId++,
      family: opts.family || "Untitled",
      style: opts.style || "Regular",     // Regular, Bold, Italic, Bold Italic, Light, etc.
      postscriptName: opts.postscriptName || (opts.family || "Untitled").replace(/\s/g, "") + "-" + (opts.style || "Regular"),
      category: opts.category || "sans-serif",  // serif, sans-serif, monospace, display, handwriting
      fileType: opts.fileType || "otf",    // otf, ttf, woff, woff2
      fileSize: opts.fileSize || 0,
      copyright: opts.copyright || "",
      version: opts.version || "1.0",
      installedAt: Date.now(),
      enabled: true,
    };
    this.fonts.push(font);
    // Add to "All Fonts" and "Recently Added"
    this.collections[0].fontIds.push(font.id);
    this.collections[1].fontIds.push(font.id);
    return font;
  }

  getFont(id) {
    return this.fonts.find(function (f) { return f.id === id; }) || null;
  }

  getFontByFamily(family) {
    return this.fonts.filter(function (f) { return f.family === family; });
  }

  getAllFonts() {
    return this.fonts.filter(function (f) { return f.enabled; });
  }

  getAllFontsIncludingDisabled() {
    return this.fonts.slice();
  }

  removeFont(id) {
    var idx = this.fonts.findIndex(function (f) { return f.id === id; });
    if (idx === -1) return false;
    this.fonts.splice(idx, 1);
    // Remove from all collections
    this.collections.forEach(function (c) {
      var ci = c.fontIds.indexOf(id);
      if (ci !== -1) c.fontIds.splice(ci, 1);
    });
    // Remove from favorites/disabled
    var fi = this.favorites.indexOf(id);
    if (fi !== -1) this.favorites.splice(fi, 1);
    var di = this.disabled.indexOf(id);
    if (di !== -1) this.disabled.splice(di, 1);
    return true;
  }

  /* ── Enable / Disable ─────────── */
  disableFont(id) {
    var font = this.getFont(id);
    if (!font) return false;
    font.enabled = false;
    if (this.disabled.indexOf(id) === -1) this.disabled.push(id);
    return true;
  }

  enableFont(id) {
    var font = this.getFont(id);
    if (!font) return false;
    font.enabled = true;
    var di = this.disabled.indexOf(id);
    if (di !== -1) this.disabled.splice(di, 1);
    return true;
  }

  getDisabledFonts() {
    var self = this;
    return this.disabled.map(function (id) { return self.getFont(id); }).filter(Boolean);
  }

  /* ── Collections ──────────────── */
  createCollection(name) {
    if (!name) return null;
    var coll = { id: this.nextCollId++, name: name, system: false, fontIds: [] };
    this.collections.push(coll);
    return coll;
  }

  getCollection(id) {
    return this.collections.find(function (c) { return c.id === id; }) || null;
  }

  getAllCollections() {
    return this.collections.slice();
  }

  deleteCollection(id) {
    var coll = this.getCollection(id);
    if (!coll || coll.system) return false;  // can't delete system collections
    var idx = this.collections.findIndex(function (c) { return c.id === id; });
    this.collections.splice(idx, 1);
    return true;
  }

  renameCollection(id, name) {
    var coll = this.getCollection(id);
    if (!coll || coll.system) return false;
    coll.name = name;
    return true;
  }

  addFontToCollection(collId, fontId) {
    var coll = this.getCollection(collId);
    if (!coll || !this.getFont(fontId)) return false;
    if (coll.fontIds.indexOf(fontId) !== -1) return false;
    coll.fontIds.push(fontId);
    return true;
  }

  removeFontFromCollection(collId, fontId) {
    var coll = this.getCollection(collId);
    if (!coll) return false;
    var idx = coll.fontIds.indexOf(fontId);
    if (idx === -1) return false;
    coll.fontIds.splice(idx, 1);
    return true;
  }

  getCollectionFonts(collId) {
    var coll = this.getCollection(collId);
    if (!coll) return [];
    var self = this;
    return coll.fontIds.map(function (id) { return self.getFont(id); }).filter(Boolean);
  }

  /* ── Favorites ────────────────── */
  toggleFavorite(id) {
    var idx = this.favorites.indexOf(id);
    if (idx !== -1) {
      this.favorites.splice(idx, 1);
      return false;
    }
    if (this.getFont(id)) {
      this.favorites.push(id);
      return true;
    }
    return false;
  }

  isFavorite(id) {
    return this.favorites.indexOf(id) !== -1;
  }

  getFavorites() {
    var self = this;
    return this.favorites.map(function (id) { return self.getFont(id); }).filter(Boolean);
  }

  /* ── Preview ──────────────────── */
  setPreviewText(text) {
    this.previewText = text;
  }

  setPreviewSize(size) {
    if (size < 8) size = 8;
    if (size > 288) size = 288;
    this.previewSize = size;
  }

  getPreview(fontId) {
    var font = this.getFont(fontId);
    if (!font) return null;
    return {
      family: font.family,
      style: font.style,
      text: this.previewText,
      size: this.previewSize,
      css: "font-family: '" + font.family + "'; font-size: " + this.previewSize + "px;",
    };
  }

  /* ── Search & Filter ──────────── */
  searchFonts(query) {
    var q = query.toLowerCase();
    return this.fonts.filter(function (f) {
      return f.family.toLowerCase().indexOf(q) !== -1 ||
             f.style.toLowerCase().indexOf(q) !== -1 ||
             f.category.toLowerCase().indexOf(q) !== -1;
    });
  }

  getFontsByCategory(category) {
    return this.fonts.filter(function (f) { return f.category === category; });
  }

  getFamilies() {
    var families = {};
    this.fonts.forEach(function (f) {
      if (!families[f.family]) families[f.family] = [];
      families[f.family].push(f);
    });
    return families;
  }

  /* ── Stats ────────────────────── */
  getStats() {
    var families = {};
    this.fonts.forEach(function (f) { families[f.family] = true; });
    return {
      totalFonts: this.fonts.length,
      totalFamilies: Object.keys(families).length,
      enabled: this.fonts.filter(function (f) { return f.enabled; }).length,
      disabled: this.disabled.length,
      favorites: this.favorites.length,
      collections: this.collections.length,
    };
  }
}

if (typeof module !== "undefined") module.exports = { FontBook: FontBook };
