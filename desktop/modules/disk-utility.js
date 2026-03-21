/* ── Disk Utility ──────────────────────────────────── */
/* macOS-style disk info, partitions, storage breakdown */

class DiskUtility {
  constructor() {
    this.disks = [];
    this.nextId = 1;
  }

  /* ── Disk Management ──────────── */
  addDisk(opts) {
    var disk = {
      id: this.nextId++,
      name: opts.name || "Untitled",
      type: opts.type || "internal",       // internal, external, network
      fileSystem: opts.fileSystem || "APFS",
      totalSize: opts.totalSize || 0,       // bytes
      usedSize: opts.usedSize || 0,
      partitions: [],
      mounted: opts.mounted !== false,
      smartStatus: opts.smartStatus || "Verified",
      serialNumber: opts.serialNumber || "SN-" + this.nextId,
    };
    this.disks.push(disk);
    return disk;
  }

  getDisk(id) {
    return this.disks.find(function (d) { return d.id === id; }) || null;
  }

  getAllDisks() {
    return this.disks.slice();
  }

  removeDisk(id) {
    var idx = this.disks.findIndex(function (d) { return d.id === id; });
    if (idx === -1) return false;
    this.disks.splice(idx, 1);
    return true;
  }

  getDisksByType(type) {
    return this.disks.filter(function (d) { return d.type === type; });
  }

  /* ── Partitions ───────────────── */
  addPartition(diskId, opts) {
    var disk = this.getDisk(diskId);
    if (!disk) return null;
    var partition = {
      id: this.nextId++,
      name: opts.name || "Partition",
      fileSystem: opts.fileSystem || disk.fileSystem,
      size: opts.size || 0,
      usedSize: opts.usedSize || 0,
      mountPoint: opts.mountPoint || "/Volumes/" + (opts.name || "Partition"),
    };
    // Check total partition size doesn't exceed disk size
    var totalPartitioned = disk.partitions.reduce(function (s, p) { return s + p.size; }, 0);
    if (totalPartitioned + partition.size > disk.totalSize) return null;
    disk.partitions.push(partition);
    return partition;
  }

  getPartitions(diskId) {
    var disk = this.getDisk(diskId);
    return disk ? disk.partitions.slice() : [];
  }

  removePartition(diskId, partId) {
    var disk = this.getDisk(diskId);
    if (!disk) return false;
    var idx = disk.partitions.findIndex(function (p) { return p.id === partId; });
    if (idx === -1) return false;
    disk.partitions.splice(idx, 1);
    return true;
  }

  /* ── Storage Analysis ─────────── */
  getStorageBreakdown(diskId) {
    var disk = this.getDisk(diskId);
    if (!disk) return null;
    var free = disk.totalSize - disk.usedSize;
    var pct = disk.totalSize > 0 ? (disk.usedSize / disk.totalSize) * 100 : 0;
    return {
      total: disk.totalSize,
      used: disk.usedSize,
      free: free,
      usedPercent: Math.round(pct * 100) / 100,
      freePercent: Math.round((100 - pct) * 100) / 100,
    };
  }

  getTotalStorage() {
    var total = 0, used = 0;
    this.disks.forEach(function (d) {
      total += d.totalSize;
      used += d.usedSize;
    });
    return { total: total, used: used, free: total - used };
  }

  /* ── Disk Operations ──────────── */
  mount(id) {
    var disk = this.getDisk(id);
    if (!disk) return false;
    disk.mounted = true;
    return true;
  }

  unmount(id) {
    var disk = this.getDisk(id);
    if (!disk) return false;
    disk.mounted = false;
    return true;
  }

  erase(diskId, opts) {
    var disk = this.getDisk(diskId);
    if (!disk) return false;
    disk.name = (opts && opts.name) || disk.name;
    disk.fileSystem = (opts && opts.fileSystem) || disk.fileSystem;
    disk.usedSize = 0;
    disk.partitions = [];
    return true;
  }

  repair(diskId) {
    var disk = this.getDisk(diskId);
    if (!disk) return null;
    return {
      diskId: diskId,
      status: disk.smartStatus === "Verified" ? "OK" : "Errors Found",
      timestamp: Date.now(),
    };
  }

  /* ── Formatting ───────────────── */
  formatSize(bytes) {
    if (bytes === 0) return "0 B";
    var units = ["B", "KB", "MB", "GB", "TB"];
    var i = Math.floor(Math.log(bytes) / Math.log(1024));
    if (i >= units.length) i = units.length - 1;
    return (bytes / Math.pow(1024, i)).toFixed(1) + " " + units[i];
  }
}

if (typeof module !== "undefined") module.exports = { DiskUtility: DiskUtility };
