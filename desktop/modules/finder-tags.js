/* ── Finder Tags ──────────────────────────────────── */
/* macOS-style color tags for file organization */

class FinderTags {
  constructor() {
    this.tags = [
      { id: 1, name: "Red",    color: "#ff3b30" },
      { id: 2, name: "Orange", color: "#ff9500" },
      { id: 3, name: "Yellow", color: "#ffcc00" },
      { id: 4, name: "Green",  color: "#34c759" },
      { id: 5, name: "Blue",   color: "#007aff" },
      { id: 6, name: "Purple", color: "#af52de" },
      { id: 7, name: "Gray",   color: "#8e8e93" },
    ];
    this.nextTagId = 8;
    this.files = [];     // { path, name, tags: [tagId, ...], dateModified }
    this.nextFileId = 1;
  }

  /* ── Tag Management ───────────── */
  createTag(name, color) {
    if (!name) return null;
    var existing = this.tags.find(function (t) { return t.name.toLowerCase() === name.toLowerCase(); });
    if (existing) return existing;
    var tag = { id: this.nextTagId++, name: name, color: color || "#8e8e93" };
    this.tags.push(tag);
    return tag;
  }

  getTag(id) {
    return this.tags.find(function (t) { return t.id === id; }) || null;
  }

  getTagByName(name) {
    return this.tags.find(function (t) { return t.name.toLowerCase() === name.toLowerCase(); }) || null;
  }

  getAllTags() {
    return this.tags.slice();
  }

  deleteTag(id) {
    var idx = this.tags.findIndex(function (t) { return t.id === id; });
    if (idx === -1) return false;
    // Remove tag from all files
    this.files.forEach(function (f) {
      var ti = f.tags.indexOf(id);
      if (ti !== -1) f.tags.splice(ti, 1);
    });
    this.tags.splice(idx, 1);
    return true;
  }

  renameTag(id, newName) {
    var tag = this.getTag(id);
    if (!tag) return null;
    tag.name = newName;
    return tag;
  }

  recolorTag(id, newColor) {
    var tag = this.getTag(id);
    if (!tag) return null;
    tag.color = newColor;
    return tag;
  }

  /* ── File Management ──────────── */
  addFile(path, name) {
    var file = {
      id: this.nextFileId++,
      path: path,
      name: name || path.split("/").pop(),
      tags: [],
      dateModified: Date.now(),
    };
    this.files.push(file);
    return file;
  }

  getFile(id) {
    return this.files.find(function (f) { return f.id === id; }) || null;
  }

  getFileByPath(path) {
    return this.files.find(function (f) { return f.path === path; }) || null;
  }

  removeFile(id) {
    var idx = this.files.findIndex(function (f) { return f.id === id; });
    if (idx === -1) return false;
    this.files.splice(idx, 1);
    return true;
  }

  /* ── Tagging Operations ───────── */
  tagFile(fileId, tagId) {
    var file = this.getFile(fileId);
    var tag = this.getTag(tagId);
    if (!file || !tag) return false;
    if (file.tags.indexOf(tagId) !== -1) return false; // already tagged
    file.tags.push(tagId);
    file.dateModified = Date.now();
    return true;
  }

  untagFile(fileId, tagId) {
    var file = this.getFile(fileId);
    if (!file) return false;
    var idx = file.tags.indexOf(tagId);
    if (idx === -1) return false;
    file.tags.splice(idx, 1);
    file.dateModified = Date.now();
    return true;
  }

  getFileTags(fileId) {
    var file = this.getFile(fileId);
    if (!file) return [];
    var self = this;
    return file.tags.map(function (tid) { return self.getTag(tid); }).filter(Boolean);
  }

  setFileTags(fileId, tagIds) {
    var file = this.getFile(fileId);
    if (!file) return false;
    var self = this;
    file.tags = tagIds.filter(function (id) { return self.getTag(id) !== null; });
    file.dateModified = Date.now();
    return true;
  }

  /* ── Search & Filter ──────────── */
  getFilesByTag(tagId) {
    return this.files.filter(function (f) { return f.tags.indexOf(tagId) !== -1; });
  }

  getFilesByTags(tagIds, matchAll) {
    return this.files.filter(function (f) {
      if (matchAll) {
        return tagIds.every(function (tid) { return f.tags.indexOf(tid) !== -1; });
      }
      return tagIds.some(function (tid) { return f.tags.indexOf(tid) !== -1; });
    });
  }

  getUntaggedFiles() {
    return this.files.filter(function (f) { return f.tags.length === 0; });
  }

  searchFiles(query) {
    var q = query.toLowerCase();
    return this.files.filter(function (f) {
      return f.name.toLowerCase().indexOf(q) !== -1 || f.path.toLowerCase().indexOf(q) !== -1;
    });
  }

  /* ── Tag Statistics ───────────── */
  getTagCounts() {
    var counts = {};
    var self = this;
    this.tags.forEach(function (t) { counts[t.id] = 0; });
    this.files.forEach(function (f) {
      f.tags.forEach(function (tid) {
        if (counts[tid] !== undefined) counts[tid]++;
      });
    });
    return counts;
  }
}

if (typeof module !== "undefined") module.exports = { FinderTags: FinderTags };
