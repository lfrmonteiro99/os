const { TimeMachine } = require("../modules/time-machine");

describe("TimeMachine", () => {
  let tm;
  beforeEach(() => { tm = new TimeMachine({ maxBackups: 5 }); });

  /* ── Configuration ──────────── */
  describe("Configuration", () => {
    test("starts disabled", () => {
      expect(tm.isEnabled()).toBe(false);
    });

    test("enable requires disk name", () => {
      expect(tm.enable()).toBe(false);
      expect(tm.enable("Backup HD")).toBe(true);
      expect(tm.isEnabled()).toBe(true);
    });

    test("disable stops backups", () => {
      tm.enable("Disk");
      tm.disable();
      expect(tm.isEnabled()).toBe(false);
    });

    test("setBackupDisk/getBackupDisk", () => {
      tm.setBackupDisk("External");
      expect(tm.getBackupDisk()).toBe("External");
    });

    test("exclusions can be added/removed", () => {
      tm.addExclusion("/tmp");
      tm.addExclusion("/cache");
      expect(tm.getExclusions()).toHaveLength(2);
      tm.removeExclusion("/tmp");
      expect(tm.getExclusions()).toEqual(["/cache"]);
    });

    test("addExclusion ignores duplicates", () => {
      tm.addExclusion("/tmp");
      tm.addExclusion("/tmp");
      expect(tm.getExclusions()).toHaveLength(1);
    });
  });

  /* ── Backups ────────────────── */
  describe("Backup Management", () => {
    beforeEach(() => { tm.enable("Disk"); });

    test("createBackup when enabled", () => {
      const b = tm.createBackup({ size: 1000, files: ["/a.txt", "/b.txt"] });
      expect(b).not.toBeNull();
      expect(b.files).toHaveLength(2);
      expect(b.status).toBe("completed");
    });

    test("createBackup returns null when disabled", () => {
      tm.disable();
      expect(tm.createBackup({})).toBeNull();
    });

    test("getBackup retrieves by id", () => {
      const b = tm.createBackup({ label: "test" });
      expect(tm.getBackup(b.id).label).toBe("test");
    });

    test("getAllBackups returns all", () => {
      tm.createBackup({});
      tm.createBackup({});
      expect(tm.getAllBackups()).toHaveLength(2);
    });

    test("getLatestBackup returns most recent", () => {
      tm.createBackup({ label: "old" });
      tm.createBackup({ label: "new" });
      expect(tm.getLatestBackup().label).toBe("new");
    });

    test("getLatestBackup returns null when empty", () => {
      expect(tm.getLatestBackup()).toBeNull();
    });

    test("deleteBackup removes backup", () => {
      const b = tm.createBackup({ size: 500 });
      expect(tm.deleteBackup(b.id)).toBe(true);
      expect(tm.getAllBackups()).toHaveLength(0);
      expect(tm.getStorageUsed()).toBe(0);
    });

    test("enforces maxBackups limit", () => {
      for (let i = 0; i < 7; i++) tm.createBackup({ size: 100 });
      expect(tm.getAllBackups()).toHaveLength(5);
    });
  });

  /* ── Timeline ───────────────── */
  describe("Timeline", () => {
    beforeEach(() => { tm.enable("Disk"); });

    test("getTimeline returns summary entries", () => {
      tm.createBackup({ label: "a" });
      tm.createBackup({ label: "b" });
      const tl = tm.getTimeline();
      expect(tl).toHaveLength(2);
      expect(tl[0]).toHaveProperty("id");
      expect(tl[0]).toHaveProperty("timestamp");
    });

    test("getBackupsByDateRange filters by time", () => {
      const now = Date.now();
      tm.createBackup({ timestamp: now - 10000 });
      tm.createBackup({ timestamp: now });
      tm.createBackup({ timestamp: now + 10000 });
      expect(tm.getBackupsByDateRange(now - 5000, now + 5000)).toHaveLength(1);
    });

    test("getBackupsGroupedByDay groups correctly", () => {
      const d1 = new Date("2024-06-15T10:00:00Z").getTime();
      const d2 = new Date("2024-06-15T14:00:00Z").getTime();
      const d3 = new Date("2024-06-16T10:00:00Z").getTime();
      tm.createBackup({ timestamp: d1 });
      tm.createBackup({ timestamp: d2 });
      tm.createBackup({ timestamp: d3 });
      const groups = tm.getBackupsGroupedByDay();
      expect(Object.keys(groups)).toHaveLength(2);
      expect(groups["2024-06-15"]).toHaveLength(2);
    });
  });

  /* ── Restore ────────────────── */
  describe("Restore", () => {
    beforeEach(() => { tm.enable("Disk"); });

    test("restoreBackup returns restore info", () => {
      const b = tm.createBackup({ files: ["/doc.txt", "/img.png"] });
      const r = tm.restoreBackup(b.id);
      expect(r.status).toBe("restored");
      expect(r.files).toHaveLength(2);
    });

    test("restoreBackup returns null for missing", () => {
      expect(tm.restoreBackup(999)).toBeNull();
    });

    test("restoreFile restores single file", () => {
      const b = tm.createBackup({ files: ["/a.txt", "/b.txt"] });
      const r = tm.restoreFile(b.id, "/a.txt");
      expect(r.file).toBe("/a.txt");
      expect(r.status).toBe("restored");
    });

    test("restoreFile returns null for missing file", () => {
      const b = tm.createBackup({ files: ["/a.txt"] });
      expect(tm.restoreFile(b.id, "/nope.txt")).toBeNull();
    });

    test("searchFile finds file across backups", () => {
      tm.createBackup({ files: ["/docs/report.pdf", "/img.png"] });
      tm.createBackup({ files: ["/docs/report.pdf"] });
      expect(tm.searchFile("report")).toHaveLength(2);
    });
  });

  /* ── Storage ────────────────── */
  describe("Storage Info", () => {
    beforeEach(() => { tm.enable("Disk"); });

    test("getStorageUsed tracks total size", () => {
      tm.createBackup({ size: 1000 });
      tm.createBackup({ size: 2000 });
      expect(tm.getStorageUsed()).toBe(3000);
    });

    test("getStorageInfo returns summary", () => {
      tm.createBackup({ size: 1000 });
      const info = tm.getStorageInfo();
      expect(info.used).toBe(1000);
      expect(info.backupCount).toBe(1);
      expect(info.remaining).toBe(tm.maxSize - 1000);
    });
  });

  /* ── Labels ─────────────────── */
  describe("Labels", () => {
    beforeEach(() => { tm.enable("Disk"); });

    test("labelBackup adds label", () => {
      const b = tm.createBackup({});
      expect(tm.labelBackup(b.id, "Before update")).toBe(true);
      expect(tm.getBackup(b.id).label).toBe("Before update");
    });

    test("labelBackup returns false for missing", () => {
      expect(tm.labelBackup(999, "x")).toBe(false);
    });
  });
});
