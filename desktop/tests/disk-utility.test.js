const { DiskUtility } = require("../modules/disk-utility");

describe("DiskUtility", () => {
  let du;
  beforeEach(() => { du = new DiskUtility(); });

  /* ── Disk Management ────────── */
  describe("Disk CRUD", () => {
    test("addDisk creates a disk with defaults", () => {
      const d = du.addDisk({ name: "Macintosh HD", totalSize: 500e9 });
      expect(d.name).toBe("Macintosh HD");
      expect(d.totalSize).toBe(500e9);
      expect(d.fileSystem).toBe("APFS");
      expect(d.mounted).toBe(true);
    });

    test("getDisk retrieves by id", () => {
      const d = du.addDisk({ name: "Test" });
      expect(du.getDisk(d.id).name).toBe("Test");
    });

    test("getDisk returns null for missing", () => {
      expect(du.getDisk(999)).toBeNull();
    });

    test("getAllDisks returns all", () => {
      du.addDisk({ name: "A" });
      du.addDisk({ name: "B" });
      expect(du.getAllDisks()).toHaveLength(2);
    });

    test("removeDisk deletes a disk", () => {
      const d = du.addDisk({ name: "X" });
      expect(du.removeDisk(d.id)).toBe(true);
      expect(du.getAllDisks()).toHaveLength(0);
    });

    test("removeDisk returns false for missing", () => {
      expect(du.removeDisk(999)).toBe(false);
    });

    test("getDisksByType filters correctly", () => {
      du.addDisk({ name: "Int", type: "internal" });
      du.addDisk({ name: "Ext", type: "external" });
      expect(du.getDisksByType("external")).toHaveLength(1);
    });
  });

  /* ── Partitions ─────────────── */
  describe("Partitions", () => {
    test("addPartition creates partition", () => {
      const d = du.addDisk({ name: "HD", totalSize: 500e9 });
      const p = du.addPartition(d.id, { name: "Data", size: 200e9 });
      expect(p.name).toBe("Data");
      expect(p.mountPoint).toBe("/Volumes/Data");
    });

    test("addPartition rejects if exceeds disk size", () => {
      const d = du.addDisk({ name: "HD", totalSize: 100 });
      du.addPartition(d.id, { name: "A", size: 80 });
      expect(du.addPartition(d.id, { name: "B", size: 30 })).toBeNull();
    });

    test("getPartitions returns all partitions for disk", () => {
      const d = du.addDisk({ name: "HD", totalSize: 500e9 });
      du.addPartition(d.id, { name: "A", size: 100e9 });
      du.addPartition(d.id, { name: "B", size: 100e9 });
      expect(du.getPartitions(d.id)).toHaveLength(2);
    });

    test("removePartition deletes partition", () => {
      const d = du.addDisk({ name: "HD", totalSize: 500e9 });
      const p = du.addPartition(d.id, { name: "A", size: 100e9 });
      expect(du.removePartition(d.id, p.id)).toBe(true);
      expect(du.getPartitions(d.id)).toHaveLength(0);
    });
  });

  /* ── Storage Analysis ───────── */
  describe("Storage", () => {
    test("getStorageBreakdown calculates correctly", () => {
      const d = du.addDisk({ name: "HD", totalSize: 1000, usedSize: 400 });
      const b = du.getStorageBreakdown(d.id);
      expect(b.total).toBe(1000);
      expect(b.used).toBe(400);
      expect(b.free).toBe(600);
      expect(b.usedPercent).toBe(40);
    });

    test("getTotalStorage sums all disks", () => {
      du.addDisk({ totalSize: 500, usedSize: 200 });
      du.addDisk({ totalSize: 300, usedSize: 100 });
      const t = du.getTotalStorage();
      expect(t.total).toBe(800);
      expect(t.used).toBe(300);
      expect(t.free).toBe(500);
    });
  });

  /* ── Operations ─────────────── */
  describe("Disk Operations", () => {
    test("mount/unmount toggles mounted state", () => {
      const d = du.addDisk({ name: "HD" });
      du.unmount(d.id);
      expect(du.getDisk(d.id).mounted).toBe(false);
      du.mount(d.id);
      expect(du.getDisk(d.id).mounted).toBe(true);
    });

    test("erase resets disk data", () => {
      const d = du.addDisk({ name: "HD", totalSize: 500e9, usedSize: 200e9 });
      du.addPartition(d.id, { name: "A", size: 100e9 });
      du.erase(d.id, { name: "Clean HD", fileSystem: "HFS+" });
      expect(du.getDisk(d.id).usedSize).toBe(0);
      expect(du.getDisk(d.id).name).toBe("Clean HD");
      expect(du.getPartitions(d.id)).toHaveLength(0);
    });

    test("repair returns status", () => {
      const d = du.addDisk({ name: "HD", smartStatus: "Verified" });
      const r = du.repair(d.id);
      expect(r.status).toBe("OK");
    });

    test("repair reports errors when SMART fails", () => {
      const d = du.addDisk({ name: "Bad", smartStatus: "Failing" });
      expect(du.repair(d.id).status).toBe("Errors Found");
    });
  });

  /* ── Formatting ─────────────── */
  describe("formatSize", () => {
    test("formats bytes correctly", () => {
      expect(du.formatSize(0)).toBe("0 B");
      expect(du.formatSize(1024)).toBe("1.0 KB");
      expect(du.formatSize(1048576)).toBe("1.0 MB");
      expect(du.formatSize(1073741824)).toBe("1.0 GB");
    });
  });
});
