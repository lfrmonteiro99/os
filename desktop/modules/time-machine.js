/* ── Time Machine ─────────────────────────────────── */
/* macOS-style backup snapshots with restore and timeline */

class TimeMachine {
  constructor(opts) {
    opts = opts || {};
    this.backups = [];
    this.nextId = 1;
    this.enabled = false;
    this.backupDisk = null;         // target disk name
    this.excludedPaths = [];
    this.backupInterval = opts.backupInterval || 3600000; // 1 hour default
    this.maxBackups = opts.maxBackups || 100;
    this.totalSize = 0;
    this.maxSize = opts.maxSize || 500 * 1024 * 1024 * 1024; // 500GB
  }

  /* ── Configuration ────────────── */
  enable(diskName) {
    this.backupDisk = diskName || this.backupDisk;
    if (!this.backupDisk) return false;
    this.enabled = true;
    return true;
  }

  disable() {
    this.enabled = false;
    return true;
  }

  isEnabled() {
    return this.enabled;
  }

  setBackupDisk(name) {
    this.backupDisk = name;
    return true;
  }

  getBackupDisk() {
    return this.backupDisk;
  }

  addExclusion(path) {
    if (this.excludedPaths.indexOf(path) === -1) {
      this.excludedPaths.push(path);
    }
    return this.excludedPaths.slice();
  }

  removeExclusion(path) {
    var idx = this.excludedPaths.indexOf(path);
    if (idx !== -1) this.excludedPaths.splice(idx, 1);
    return this.excludedPaths.slice();
  }

  getExclusions() {
    return this.excludedPaths.slice();
  }

  /* ── Backup Management ────────── */
  createBackup(opts) {
    if (!this.enabled) return null;
    opts = opts || {};
    var backup = {
      id: this.nextId++,
      timestamp: opts.timestamp || Date.now(),
      size: opts.size || 0,
      files: opts.files || [],          // list of file paths
      type: opts.type || "auto",        // auto, manual
      status: "completed",
      label: opts.label || null,
    };
    this.backups.push(backup);
    this.totalSize += backup.size;

    // Enforce max backups — remove oldest first
    while (this.backups.length > this.maxBackups) {
      var removed = this.backups.shift();
      this.totalSize -= removed.size;
    }

    return backup;
  }

  getBackup(id) {
    return this.backups.find(function (b) { return b.id === id; }) || null;
  }

  getAllBackups() {
    return this.backups.slice();
  }

  getLatestBackup() {
    if (this.backups.length === 0) return null;
    return this.backups[this.backups.length - 1];
  }

  deleteBackup(id) {
    var idx = this.backups.findIndex(function (b) { return b.id === id; });
    if (idx === -1) return false;
    this.totalSize -= this.backups[idx].size;
    this.backups.splice(idx, 1);
    return true;
  }

  /* ── Timeline ─────────────────── */
  getTimeline() {
    return this.backups.map(function (b) {
      return { id: b.id, timestamp: b.timestamp, size: b.size, type: b.type, label: b.label };
    });
  }

  getBackupsByDateRange(start, end) {
    return this.backups.filter(function (b) {
      return b.timestamp >= start && b.timestamp <= end;
    });
  }

  getBackupsGroupedByDay() {
    var groups = {};
    this.backups.forEach(function (b) {
      var day = new Date(b.timestamp).toISOString().slice(0, 10);
      if (!groups[day]) groups[day] = [];
      groups[day].push(b);
    });
    return groups;
  }

  /* ── Restore ──────────────────── */
  restoreBackup(id) {
    var backup = this.getBackup(id);
    if (!backup) return null;
    return {
      backupId: id,
      timestamp: backup.timestamp,
      files: backup.files.slice(),
      restoredAt: Date.now(),
      status: "restored",
    };
  }

  restoreFile(backupId, filePath) {
    var backup = this.getBackup(backupId);
    if (!backup) return null;
    var found = backup.files.indexOf(filePath) !== -1;
    if (!found) return null;
    return {
      backupId: backupId,
      file: filePath,
      restoredAt: Date.now(),
      status: "restored",
    };
  }

  searchFile(fileName) {
    var results = [];
    this.backups.forEach(function (b) {
      b.files.forEach(function (f) {
        if (f.indexOf(fileName) !== -1) {
          results.push({ backupId: b.id, timestamp: b.timestamp, file: f });
        }
      });
    });
    return results;
  }

  /* ── Storage ──────────────────── */
  getStorageUsed() {
    return this.totalSize;
  }

  getStorageInfo() {
    return {
      used: this.totalSize,
      max: this.maxSize,
      remaining: this.maxSize - this.totalSize,
      usedPercent: this.maxSize > 0 ? Math.round((this.totalSize / this.maxSize) * 10000) / 100 : 0,
      backupCount: this.backups.length,
    };
  }

  /* ── Labels ───────────────────── */
  labelBackup(id, label) {
    var backup = this.getBackup(id);
    if (!backup) return false;
    backup.label = label;
    return true;
  }
}

if (typeof module !== "undefined") module.exports = { TimeMachine: TimeMachine };
