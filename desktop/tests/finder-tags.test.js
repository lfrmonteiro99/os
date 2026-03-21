const { FinderTags } = require("../modules/finder-tags");

describe("FinderTags", () => {
  let ft;
  beforeEach(() => { ft = new FinderTags(); });

  /* ── Default Tags ───────────── */
  describe("Built-in Tags", () => {
    test("starts with 7 default color tags", () => {
      expect(ft.getAllTags()).toHaveLength(7);
    });

    test("default tags have correct colors", () => {
      const red = ft.getTagByName("Red");
      expect(red).not.toBeNull();
      expect(red.color).toBe("#ff3b30");
    });
  });

  /* ── Tag CRUD ───────────────── */
  describe("Tag Management", () => {
    test("createTag adds new tag", () => {
      const t = ft.createTag("Work", "#0000ff");
      expect(t.name).toBe("Work");
      expect(ft.getAllTags()).toHaveLength(8);
    });

    test("createTag returns existing if duplicate name", () => {
      const t = ft.createTag("Red", "#111");
      expect(t.id).toBe(1); // existing id
      expect(ft.getAllTags()).toHaveLength(7);
    });

    test("createTag rejects empty name", () => {
      expect(ft.createTag("")).toBeNull();
    });

    test("deleteTag removes tag from all files", () => {
      const f = ft.addFile("/test.txt");
      ft.tagFile(f.id, 1); // Red
      ft.deleteTag(1);
      expect(ft.getFileTags(f.id)).toHaveLength(0);
      expect(ft.getAllTags()).toHaveLength(6);
    });

    test("renameTag changes tag name", () => {
      ft.renameTag(1, "Important");
      expect(ft.getTag(1).name).toBe("Important");
    });

    test("recolorTag changes tag color", () => {
      ft.recolorTag(1, "#000000");
      expect(ft.getTag(1).color).toBe("#000000");
    });
  });

  /* ── File Management ────────── */
  describe("File Operations", () => {
    test("addFile creates file entry", () => {
      const f = ft.addFile("/Users/user/doc.txt", "doc.txt");
      expect(f.name).toBe("doc.txt");
      expect(f.tags).toEqual([]);
    });

    test("addFile infers name from path", () => {
      const f = ft.addFile("/Users/user/report.pdf");
      expect(f.name).toBe("report.pdf");
    });

    test("getFileByPath finds file", () => {
      ft.addFile("/a.txt");
      expect(ft.getFileByPath("/a.txt")).not.toBeNull();
    });

    test("removeFile deletes file", () => {
      const f = ft.addFile("/tmp.txt");
      expect(ft.removeFile(f.id)).toBe(true);
      expect(ft.getFile(f.id)).toBeNull();
    });
  });

  /* ── Tagging ────────────────── */
  describe("Tagging Operations", () => {
    test("tagFile adds tag to file", () => {
      const f = ft.addFile("/a.txt");
      expect(ft.tagFile(f.id, 1)).toBe(true);
      expect(ft.getFileTags(f.id)).toHaveLength(1);
      expect(ft.getFileTags(f.id)[0].name).toBe("Red");
    });

    test("tagFile rejects duplicate", () => {
      const f = ft.addFile("/a.txt");
      ft.tagFile(f.id, 1);
      expect(ft.tagFile(f.id, 1)).toBe(false);
    });

    test("tagFile fails for invalid file/tag", () => {
      expect(ft.tagFile(999, 1)).toBe(false);
      const f = ft.addFile("/a.txt");
      expect(ft.tagFile(f.id, 999)).toBe(false);
    });

    test("untagFile removes tag from file", () => {
      const f = ft.addFile("/a.txt");
      ft.tagFile(f.id, 1);
      expect(ft.untagFile(f.id, 1)).toBe(true);
      expect(ft.getFileTags(f.id)).toHaveLength(0);
    });

    test("setFileTags replaces all tags", () => {
      const f = ft.addFile("/a.txt");
      ft.tagFile(f.id, 1);
      ft.setFileTags(f.id, [2, 3]);
      const tags = ft.getFileTags(f.id);
      expect(tags).toHaveLength(2);
      expect(tags[0].name).toBe("Orange");
    });

    test("multiple tags per file", () => {
      const f = ft.addFile("/a.txt");
      ft.tagFile(f.id, 1);
      ft.tagFile(f.id, 5);
      ft.tagFile(f.id, 7);
      expect(ft.getFileTags(f.id)).toHaveLength(3);
    });
  });

  /* ── Search & Filter ────────── */
  describe("Search", () => {
    test("getFilesByTag returns tagged files", () => {
      const a = ft.addFile("/a.txt");
      const b = ft.addFile("/b.txt");
      ft.tagFile(a.id, 1);
      ft.tagFile(b.id, 1);
      expect(ft.getFilesByTag(1)).toHaveLength(2);
    });

    test("getFilesByTags with matchAll=true", () => {
      const a = ft.addFile("/a.txt");
      ft.tagFile(a.id, 1);
      ft.tagFile(a.id, 2);
      const b = ft.addFile("/b.txt");
      ft.tagFile(b.id, 1);
      expect(ft.getFilesByTags([1, 2], true)).toHaveLength(1);
    });

    test("getFilesByTags with matchAll=false (any)", () => {
      const a = ft.addFile("/a.txt");
      ft.tagFile(a.id, 1);
      const b = ft.addFile("/b.txt");
      ft.tagFile(b.id, 2);
      expect(ft.getFilesByTags([1, 2], false)).toHaveLength(2);
    });

    test("getUntaggedFiles returns files without tags", () => {
      ft.addFile("/a.txt");
      const b = ft.addFile("/b.txt");
      ft.tagFile(b.id, 1);
      expect(ft.getUntaggedFiles()).toHaveLength(1);
    });

    test("searchFiles matches by name", () => {
      ft.addFile("/docs/report.pdf");
      ft.addFile("/docs/notes.txt");
      expect(ft.searchFiles("report")).toHaveLength(1);
    });
  });

  /* ── Statistics ─────────────── */
  describe("Tag Counts", () => {
    test("getTagCounts returns usage per tag", () => {
      const a = ft.addFile("/a.txt");
      const b = ft.addFile("/b.txt");
      ft.tagFile(a.id, 1);
      ft.tagFile(b.id, 1);
      ft.tagFile(a.id, 2);
      const counts = ft.getTagCounts();
      expect(counts[1]).toBe(2);
      expect(counts[2]).toBe(1);
      expect(counts[3]).toBe(0);
    });
  });
});
