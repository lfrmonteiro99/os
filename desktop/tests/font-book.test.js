const { FontBook } = require("../modules/font-book");

describe("FontBook", () => {
  let fb;
  beforeEach(() => { fb = new FontBook(); });

  /* ── Font Management ────────── */
  describe("Fonts", () => {
    test("addFont creates font entry", () => {
      const f = fb.addFont({ family: "Helvetica", style: "Regular", category: "sans-serif" });
      expect(f.family).toBe("Helvetica");
      expect(f.postscriptName).toBe("Helvetica-Regular");
      expect(f.enabled).toBe(true);
    });

    test("addFont rejects duplicates (same family+style)", () => {
      fb.addFont({ family: "Arial", style: "Bold" });
      const dup = fb.addFont({ family: "Arial", style: "Bold" });
      expect(fb.getAllFontsIncludingDisabled()).toHaveLength(1);
    });

    test("getFont retrieves by id", () => {
      const f = fb.addFont({ family: "Monaco" });
      expect(fb.getFont(f.id).family).toBe("Monaco");
    });

    test("getFontByFamily returns all styles", () => {
      fb.addFont({ family: "Helvetica", style: "Regular" });
      fb.addFont({ family: "Helvetica", style: "Bold" });
      fb.addFont({ family: "Arial", style: "Regular" });
      expect(fb.getFontByFamily("Helvetica")).toHaveLength(2);
    });

    test("getAllFonts excludes disabled", () => {
      const f = fb.addFont({ family: "A" });
      fb.addFont({ family: "B" });
      fb.disableFont(f.id);
      expect(fb.getAllFonts()).toHaveLength(1);
    });

    test("removeFont deletes from everywhere", () => {
      const f = fb.addFont({ family: "X" });
      fb.toggleFavorite(f.id);
      const coll = fb.createCollection("My Coll");
      fb.addFontToCollection(coll.id, f.id);
      fb.removeFont(f.id);
      expect(fb.getFont(f.id)).toBeNull();
      expect(fb.getFavorites()).toHaveLength(0);
      expect(fb.getCollectionFonts(coll.id)).toHaveLength(0);
    });
  });

  /* ── Enable/Disable ─────────── */
  describe("Enable/Disable", () => {
    test("disableFont marks font disabled", () => {
      const f = fb.addFont({ family: "Test" });
      fb.disableFont(f.id);
      expect(fb.getFont(f.id).enabled).toBe(false);
      expect(fb.getDisabledFonts()).toHaveLength(1);
    });

    test("enableFont re-enables font", () => {
      const f = fb.addFont({ family: "Test" });
      fb.disableFont(f.id);
      fb.enableFont(f.id);
      expect(fb.getFont(f.id).enabled).toBe(true);
      expect(fb.getDisabledFonts()).toHaveLength(0);
    });
  });

  /* ── Collections ────────────── */
  describe("Collections", () => {
    test("starts with 2 system collections", () => {
      expect(fb.getAllCollections()).toHaveLength(2);
      expect(fb.getAllCollections()[0].system).toBe(true);
    });

    test("createCollection adds custom collection", () => {
      const c = fb.createCollection("Web Fonts");
      expect(c.name).toBe("Web Fonts");
      expect(c.system).toBe(false);
    });

    test("deleteCollection removes non-system", () => {
      const c = fb.createCollection("Temp");
      expect(fb.deleteCollection(c.id)).toBe(true);
    });

    test("deleteCollection rejects system collections", () => {
      expect(fb.deleteCollection(1)).toBe(false);
    });

    test("renameCollection changes name", () => {
      const c = fb.createCollection("Old");
      fb.renameCollection(c.id, "New");
      expect(fb.getCollection(c.id).name).toBe("New");
    });

    test("addFontToCollection links font", () => {
      const f = fb.addFont({ family: "Font" });
      const c = fb.createCollection("Coll");
      expect(fb.addFontToCollection(c.id, f.id)).toBe(true);
      expect(fb.getCollectionFonts(c.id)).toHaveLength(1);
    });

    test("addFontToCollection rejects duplicate", () => {
      const f = fb.addFont({ family: "Font" });
      const c = fb.createCollection("Coll");
      fb.addFontToCollection(c.id, f.id);
      expect(fb.addFontToCollection(c.id, f.id)).toBe(false);
    });

    test("removeFontFromCollection unlinks", () => {
      const f = fb.addFont({ family: "Font" });
      const c = fb.createCollection("Coll");
      fb.addFontToCollection(c.id, f.id);
      fb.removeFontFromCollection(c.id, f.id);
      expect(fb.getCollectionFonts(c.id)).toHaveLength(0);
    });

    test("new fonts auto-added to All Fonts collection", () => {
      fb.addFont({ family: "A" });
      fb.addFont({ family: "B" });
      expect(fb.getCollectionFonts(1)).toHaveLength(2); // "All Fonts" id=1
    });
  });

  /* ── Favorites ──────────────── */
  describe("Favorites", () => {
    test("toggleFavorite adds/removes", () => {
      const f = fb.addFont({ family: "Fav" });
      expect(fb.toggleFavorite(f.id)).toBe(true);
      expect(fb.isFavorite(f.id)).toBe(true);
      expect(fb.toggleFavorite(f.id)).toBe(false);
      expect(fb.isFavorite(f.id)).toBe(false);
    });

    test("getFavorites returns favorited fonts", () => {
      const a = fb.addFont({ family: "A" });
      const b = fb.addFont({ family: "B" });
      fb.toggleFavorite(a.id);
      expect(fb.getFavorites()).toHaveLength(1);
      expect(fb.getFavorites()[0].family).toBe("A");
    });
  });

  /* ── Preview ────────────────── */
  describe("Preview", () => {
    test("getPreview returns preview info", () => {
      const f = fb.addFont({ family: "Monaco" });
      const p = fb.getPreview(f.id);
      expect(p.family).toBe("Monaco");
      expect(p.text).toContain("quick brown fox");
      expect(p.css).toContain("Monaco");
    });

    test("setPreviewText changes preview", () => {
      fb.setPreviewText("Test text");
      const f = fb.addFont({ family: "A" });
      expect(fb.getPreview(f.id).text).toBe("Test text");
    });

    test("setPreviewSize clamps 8-288", () => {
      fb.setPreviewSize(4);
      expect(fb.previewSize).toBe(8);
      fb.setPreviewSize(300);
      expect(fb.previewSize).toBe(288);
      fb.setPreviewSize(48);
      expect(fb.previewSize).toBe(48);
    });
  });

  /* ── Search & Filter ────────── */
  describe("Search", () => {
    test("searchFonts matches family/style/category", () => {
      fb.addFont({ family: "Helvetica Neue", category: "sans-serif" });
      fb.addFont({ family: "Times New Roman", category: "serif" });
      expect(fb.searchFonts("helv")).toHaveLength(1);
      expect(fb.searchFonts("serif")).toHaveLength(2); // both contain "serif"
    });

    test("getFontsByCategory filters", () => {
      fb.addFont({ family: "A", category: "monospace" });
      fb.addFont({ family: "B", category: "serif" });
      fb.addFont({ family: "C", category: "monospace" });
      expect(fb.getFontsByCategory("monospace")).toHaveLength(2);
    });

    test("getFamilies groups by family", () => {
      fb.addFont({ family: "Roboto", style: "Regular" });
      fb.addFont({ family: "Roboto", style: "Bold" });
      fb.addFont({ family: "Open Sans", style: "Regular" });
      const fams = fb.getFamilies();
      expect(Object.keys(fams)).toHaveLength(2);
      expect(fams["Roboto"]).toHaveLength(2);
    });
  });

  /* ── Stats ──────────────────── */
  describe("Stats", () => {
    test("getStats returns comprehensive info", () => {
      fb.addFont({ family: "A" });
      fb.addFont({ family: "B" });
      const f = fb.addFont({ family: "C" });
      fb.disableFont(f.id);
      fb.toggleFavorite(f.id);
      const s = fb.getStats();
      expect(s.totalFonts).toBe(3);
      expect(s.totalFamilies).toBe(3);
      expect(s.enabled).toBe(2);
      expect(s.disabled).toBe(1);
      expect(s.favorites).toBe(1);
    });
  });
});
