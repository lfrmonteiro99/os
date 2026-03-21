const { SpotlightIndexer } = require("../modules/spotlight-indexer");

describe("SpotlightIndexer", () => {
  let si;
  beforeEach(() => { si = new SpotlightIndexer(); });

  /* ── Constructor ──────────────── */
  describe("Constructor", () => {
    test("starts with empty index", () => {
      expect(si.getStats().totalItems).toBe(0);
    });

    test("starts with empty search history", () => {
      expect(si.searchHistory).toEqual([]);
    });

    test("accepts custom defaultLimit option", () => {
      const custom = new SpotlightIndexer({ defaultLimit: 5 });
      expect(custom.defaultLimit).toBe(5);
    });
  });

  /* ── addItem / removeItem ────── */
  describe("Item Management", () => {
    test("addItem stores item in index", () => {
      const item = si.addItem({ id: "a1", name: "Notes.txt", type: "document", path: "/docs/Notes.txt", content: "hello world", metadata: {} });
      expect(item.id).toBe("a1");
      expect(si.getStats().totalItems).toBe(1);
    });

    test("addItem auto-generates id when missing", () => {
      const item = si.addItem({ name: "Test.txt", type: "document", path: "/Test.txt", content: "", metadata: {} });
      expect(item.id).toBeDefined();
    });

    test("removeItem deletes item by id", () => {
      si.addItem({ id: "a1", name: "Notes.txt", type: "document", path: "/docs/Notes.txt", content: "", metadata: {} });
      expect(si.removeItem("a1")).toBe(true);
      expect(si.getStats().totalItems).toBe(0);
    });

    test("removeItem returns false for unknown id", () => {
      expect(si.removeItem("nonexistent")).toBe(false);
    });
  });

  /* ── Search by name ──────────── */
  describe("Search by Name", () => {
    beforeEach(() => {
      si.addItem({ id: "1", name: "ProjectPlan.pdf", type: "document", path: "/docs/ProjectPlan.pdf", content: "", metadata: {} });
      si.addItem({ id: "2", name: "project-notes.txt", type: "document", path: "/docs/project-notes.txt", content: "", metadata: {} });
      si.addItem({ id: "3", name: "Vacation.jpg", type: "image", path: "/photos/Vacation.jpg", content: "", metadata: {} });
    });

    test("search finds partial name match", () => {
      const results = si.search("project");
      expect(results.length).toBe(2);
    });

    test("search is case insensitive", () => {
      const results = si.search("VACATION");
      expect(results.length).toBe(1);
      expect(results[0].id).toBe("3");
    });
  });

  /* ── Search by content ─────── */
  describe("Search by Content", () => {
    test("search matches content text", () => {
      si.addItem({ id: "c1", name: "readme.md", type: "document", path: "/readme.md", content: "This project uses JavaScript and Node.js", metadata: {} });
      si.addItem({ id: "c2", name: "style.css", type: "document", path: "/style.css", content: "body { color: red; }", metadata: {} });
      const results = si.search("JavaScript");
      expect(results.length).toBe(1);
      expect(results[0].id).toBe("c1");
    });
  });

  /* ── Search by type filter ───── */
  describe("Search by Type Filter", () => {
    beforeEach(() => {
      si.addItem({ id: "t1", name: "App.app", type: "app", path: "/Applications/App.app", content: "", metadata: {} });
      si.addItem({ id: "t2", name: "Doc.pdf", type: "document", path: "/Doc.pdf", content: "", metadata: {} });
      si.addItem({ id: "t3", name: "Photo.jpg", type: "image", path: "/Photo.jpg", content: "", metadata: {} });
      si.addItem({ id: "t4", name: "Projects", type: "folder", path: "/Projects", content: "", metadata: {} });
      si.addItem({ id: "t5", name: "Hey.eml", type: "email", path: "/Hey.eml", content: "", metadata: {} });
      si.addItem({ id: "t6", name: "John.vcf", type: "contact", path: "/John.vcf", content: "", metadata: {} });
      si.addItem({ id: "t7", name: "Song.mp3", type: "music", path: "/Song.mp3", content: "", metadata: {} });
    });

    test("search with type filter returns only matching type", () => {
      const results = si.search("", { type: "app" });
      expect(results.length).toBe(1);
      expect(results[0].type).toBe("app");
    });

    test("getItemsByType returns items of given type", () => {
      expect(si.getItemsByType("image")).toHaveLength(1);
      expect(si.getItemsByType("document")).toHaveLength(1);
    });
  });

  /* ── Search ranking ──────────── */
  describe("Search Ranking", () => {
    beforeEach(() => {
      si.addItem({ id: "r1", name: "report", type: "document", path: "/report", content: "", metadata: {} });
      si.addItem({ id: "r2", name: "report-final", type: "document", path: "/report-final", content: "", metadata: {} });
      si.addItem({ id: "r3", name: "annual-report-2025", type: "document", path: "/annual-report-2025", content: "", metadata: {} });
    });

    test("exact name match ranks first", () => {
      const results = si.search("report");
      expect(results[0].id).toBe("r1");
    });

    test("starts-with match ranks before contains", () => {
      const results = si.search("report");
      expect(results[1].id).toBe("r2");
      expect(results[2].id).toBe("r3");
    });
  });

  /* ── Search history ──────────── */
  describe("Search History", () => {
    test("search records query in history", () => {
      si.search("hello");
      expect(si.searchHistory).toContain("hello");
    });

    test("history limited to last 10 entries", () => {
      for (let i = 0; i < 15; i++) {
        si.search("query" + i);
      }
      expect(si.searchHistory).toHaveLength(10);
      expect(si.searchHistory[0]).toBe("query5");
    });

    test("clearSearchHistory empties history", () => {
      si.search("test");
      si.clearSearchHistory();
      expect(si.searchHistory).toHaveLength(0);
    });

    test("empty query is not recorded in history", () => {
      si.search("");
      expect(si.searchHistory).toHaveLength(0);
    });
  });

  /* ── Bulk indexing ───────────── */
  describe("Bulk Indexing", () => {
    test("addItems indexes array of items", () => {
      const items = [
        { id: "b1", name: "A.txt", type: "document", path: "/A.txt", content: "", metadata: {} },
        { id: "b2", name: "B.txt", type: "document", path: "/B.txt", content: "", metadata: {} },
        { id: "b3", name: "C.txt", type: "document", path: "/C.txt", content: "", metadata: {} },
      ];
      const added = si.addItems(items);
      expect(added).toHaveLength(3);
      expect(si.getStats().totalItems).toBe(3);
    });
  });

  /* ── updateItem ──────────────── */
  describe("Update Item", () => {
    test("updateItem merges updates into existing item", () => {
      si.addItem({ id: "u1", name: "Old.txt", type: "document", path: "/Old.txt", content: "old", metadata: {} });
      const updated = si.updateItem("u1", { name: "New.txt", content: "new" });
      expect(updated.name).toBe("New.txt");
      expect(updated.content).toBe("new");
      expect(updated.path).toBe("/Old.txt");
    });

    test("updateItem returns null for unknown id", () => {
      expect(si.updateItem("no", { name: "x" })).toBeNull();
    });
  });

  /* ── Index statistics ────────── */
  describe("Statistics", () => {
    test("getStats returns totalItems and itemsByType", () => {
      si.addItem({ id: "s1", name: "A.app", type: "app", path: "/A.app", content: "", metadata: {} });
      si.addItem({ id: "s2", name: "B.pdf", type: "document", path: "/B.pdf", content: "", metadata: {} });
      si.addItem({ id: "s3", name: "C.pdf", type: "document", path: "/C.pdf", content: "", metadata: {} });
      const stats = si.getStats();
      expect(stats.totalItems).toBe(3);
      expect(stats.itemsByType.app).toBe(1);
      expect(stats.itemsByType.document).toBe(2);
    });
  });

  /* ── Multi-term search (AND) ── */
  describe("Multi-term Search", () => {
    test("multiple terms use AND logic", () => {
      si.addItem({ id: "m1", name: "project plan", type: "document", path: "/project plan", content: "budget review", metadata: {} });
      si.addItem({ id: "m2", name: "project", type: "document", path: "/project", content: "timeline", metadata: {} });
      const results = si.search("project plan");
      expect(results.length).toBe(1);
      expect(results[0].id).toBe("m1");
    });
  });

  /* ── Exclusion patterns ──────── */
  describe("Exclusion Patterns", () => {
    test("addExclusion adds a path pattern", () => {
      si.addExclusion("/tmp");
      expect(si.exclusions.has("/tmp")).toBe(true);
    });

    test("removeExclusion removes a path pattern", () => {
      si.addExclusion("/tmp");
      si.removeExclusion("/tmp");
      expect(si.exclusions.has("/tmp")).toBe(false);
    });

    test("addItem skips items matching exclusion", () => {
      si.addExclusion("/tmp");
      const item = si.addItem({ id: "e1", name: "junk.log", type: "document", path: "/tmp/junk.log", content: "", metadata: {} });
      expect(item).toBeNull();
      expect(si.getStats().totalItems).toBe(0);
    });

    test("search excludes items in excluded paths", () => {
      si.addItem({ id: "e2", name: "keep.txt", type: "document", path: "/docs/keep.txt", content: "", metadata: {} });
      si.addItem({ id: "e3", name: "skip.txt", type: "document", path: "/cache/skip.txt", content: "", metadata: {} });
      si.addExclusion("/cache");
      const results = si.search("txt");
      expect(results.every(function (r) { return !r.path.startsWith("/cache"); })).toBe(true);
    });
  });

  /* ── Metadata search ─────────── */
  describe("Metadata Search", () => {
    test("search filters by metadata date range", () => {
      si.addItem({ id: "d1", name: "old.txt", type: "document", path: "/old.txt", content: "", metadata: { dateCreated: new Date("2024-01-01").getTime(), size: 100 } });
      si.addItem({ id: "d2", name: "new.txt", type: "document", path: "/new.txt", content: "", metadata: { dateCreated: new Date("2025-06-15").getTime(), size: 200 } });
      const results = si.search("", { metadataFilter: { dateCreated: { min: new Date("2025-01-01").getTime() } } });
      expect(results.length).toBe(1);
      expect(results[0].id).toBe("d2");
    });

    test("search filters by metadata file size", () => {
      si.addItem({ id: "sz1", name: "small.txt", type: "document", path: "/small.txt", content: "", metadata: { size: 50 } });
      si.addItem({ id: "sz2", name: "big.txt", type: "document", path: "/big.txt", content: "", metadata: { size: 5000 } });
      const results = si.search("", { metadataFilter: { size: { max: 100 } } });
      expect(results.length).toBe(1);
      expect(results[0].id).toBe("sz1");
    });
  });

  /* ── Suggestions / Autocomplete */
  describe("Suggestions", () => {
    test("getSuggestions returns names starting with partial", () => {
      si.addItem({ id: "sg1", name: "Presentation.key", type: "document", path: "/Presentation.key", content: "", metadata: {} });
      si.addItem({ id: "sg2", name: "Preview.app", type: "app", path: "/Preview.app", content: "", metadata: {} });
      si.addItem({ id: "sg3", name: "Notes.txt", type: "document", path: "/Notes.txt", content: "", metadata: {} });
      const suggestions = si.getSuggestions("pre");
      expect(suggestions).toHaveLength(2);
      expect(suggestions).toContain("Presentation.key");
      expect(suggestions).toContain("Preview.app");
    });

    test("getSuggestions is case insensitive", () => {
      si.addItem({ id: "sg4", name: "Desktop", type: "folder", path: "/Desktop", content: "", metadata: {} });
      const suggestions = si.getSuggestions("desk");
      expect(suggestions).toHaveLength(1);
    });
  });

  /* ── Re-index ────────────────── */
  describe("Re-index", () => {
    test("reindexItem replaces item data entirely", () => {
      si.addItem({ id: "ri1", name: "Draft.txt", type: "document", path: "/Draft.txt", content: "version 1", metadata: {} });
      const reindexed = si.reindexItem({ id: "ri1", name: "Final.txt", type: "document", path: "/Final.txt", content: "version 2", metadata: { size: 300 } });
      expect(reindexed.name).toBe("Final.txt");
      expect(reindexed.content).toBe("version 2");
      expect(si.getStats().totalItems).toBe(1);
    });

    test("reindexItem returns null for unknown id", () => {
      expect(si.reindexItem({ id: "nope", name: "x", type: "document", path: "/x", content: "", metadata: {} })).toBeNull();
    });
  });

  /* ── Search result limit ─────── */
  describe("Result Limit", () => {
    test("search returns at most defaultLimit results", () => {
      for (let i = 0; i < 30; i++) {
        si.addItem({ id: "lim" + i, name: "file" + i + ".txt", type: "document", path: "/file" + i + ".txt", content: "", metadata: {} });
      }
      const results = si.search("file");
      expect(results.length).toBe(20);
    });

    test("search respects custom limit option", () => {
      for (let i = 0; i < 10; i++) {
        si.addItem({ id: "cl" + i, name: "item" + i + ".txt", type: "document", path: "/item" + i + ".txt", content: "", metadata: {} });
      }
      const results = si.search("item", { limit: 3 });
      expect(results.length).toBe(3);
    });
  });
});
