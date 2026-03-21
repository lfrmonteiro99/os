/* ── Spotlight Indexer ────────────────────────────── */
/* macOS-style search indexing and ranked retrieval   */

class SpotlightIndexer {
  constructor(options) {
    if (!options) options = {};
    this.index = new Map();
    this.searchHistory = [];
    this.maxHistory = 10;
    this.exclusions = new Set();
    this.defaultLimit = options.defaultLimit || 20;
    this._nextId = 1;
  }

  /* ── Item Management ─────────── */
  addItem(item) {
    if (!item) return null;
    var id = item.id || ("auto_" + this._nextId++);
    var path = item.path || "";
    var self = this;
    var excluded = false;
    this.exclusions.forEach(function (pattern) {
      if (path.indexOf(pattern) === 0) excluded = true;
    });
    if (excluded) return null;
    var entry = {
      id: id,
      name: item.name || "",
      type: item.type || "",
      path: path,
      content: item.content || "",
      metadata: item.metadata || {},
    };
    this.index.set(id, entry);
    return entry;
  }

  removeItem(id) {
    if (!this.index.has(id)) return false;
    this.index.delete(id);
    return true;
  }

  addItems(items) {
    var self = this;
    var added = [];
    items.forEach(function (item) {
      var result = self.addItem(item);
      if (result) added.push(result);
    });
    return added;
  }

  updateItem(id, updates) {
    var existing = this.index.get(id);
    if (!existing) return null;
    var keys = Object.keys(updates);
    for (var i = 0; i < keys.length; i++) {
      existing[keys[i]] = updates[keys[i]];
    }
    return existing;
  }

  reindexItem(item) {
    if (!item || !item.id || !this.index.has(item.id)) return null;
    var entry = {
      id: item.id,
      name: item.name || "",
      type: item.type || "",
      path: item.path || "",
      content: item.content || "",
      metadata: item.metadata || {},
    };
    this.index.set(item.id, entry);
    return entry;
  }

  /* ── Search ──────────────────── */
  search(query, options) {
    if (!options) options = {};
    var type = options.type || null;
    var limit = options.limit || this.defaultLimit;
    var metadataFilter = options.metadataFilter || null;
    var self = this;

    if (query && query.trim().length > 0) {
      this.searchHistory.push(query);
      if (this.searchHistory.length > this.maxHistory) {
        this.searchHistory = this.searchHistory.slice(this.searchHistory.length - this.maxHistory);
      }
    }

    var terms = query ? query.toLowerCase().split(/\s+/).filter(function (t) { return t.length > 0; }) : [];
    var results = [];

    this.index.forEach(function (item) {
      var excluded = false;
      self.exclusions.forEach(function (pattern) {
        if (item.path.indexOf(pattern) === 0) excluded = true;
      });
      if (excluded) return;

      if (type && item.type !== type) return;

      if (metadataFilter) {
        var filterKeys = Object.keys(metadataFilter);
        for (var f = 0; f < filterKeys.length; f++) {
          var key = filterKeys[f];
          var constraint = metadataFilter[key];
          var val = item.metadata[key];
          if (val === undefined) return;
          if (constraint.min !== undefined && val < constraint.min) return;
          if (constraint.max !== undefined && val > constraint.max) return;
        }
      }

      if (terms.length === 0) {
        results.push({ item: item, score: 0 });
        return;
      }

      var nameLower = item.name.toLowerCase();
      var contentLower = item.content.toLowerCase();
      var pathLower = item.path.toLowerCase();

      var allMatch = true;
      var bestScore = 0;
      for (var i = 0; i < terms.length; i++) {
        var term = terms[i];
        var inName = nameLower.indexOf(term) !== -1;
        var inContent = contentLower.indexOf(term) !== -1;
        var inPath = pathLower.indexOf(term) !== -1;
        if (!inName && !inContent && !inPath) {
          allMatch = false;
          break;
        }
        if (nameLower === term) {
          bestScore = Math.max(bestScore, 3);
        } else if (nameLower.indexOf(term) === 0) {
          bestScore = Math.max(bestScore, 2);
        } else if (inName) {
          bestScore = Math.max(bestScore, 1);
        } else {
          bestScore = Math.max(bestScore, 0);
        }
      }

      if (allMatch) {
        results.push({ item: item, score: bestScore });
      }
    });

    results.sort(function (a, b) { return b.score - a.score; });

    var limited = results.slice(0, limit);
    return limited.map(function (r) { return r.item; });
  }

  clearSearchHistory() {
    this.searchHistory = [];
  }

  /* ── Type Queries ────────────── */
  getItemsByType(type) {
    var result = [];
    this.index.forEach(function (item) {
      if (item.type === type) result.push(item);
    });
    return result;
  }

  /* ── Exclusions ──────────────── */
  addExclusion(pattern) {
    this.exclusions.add(pattern);
  }

  removeExclusion(pattern) {
    this.exclusions.delete(pattern);
  }

  /* ── Suggestions ─────────────── */
  getSuggestions(partial) {
    var p = partial.toLowerCase();
    var suggestions = [];
    this.index.forEach(function (item) {
      if (item.name.toLowerCase().indexOf(p) === 0) {
        suggestions.push(item.name);
      }
    });
    return suggestions;
  }

  /* ── Statistics ──────────────── */
  getStats() {
    var itemsByType = {};
    this.index.forEach(function (item) {
      if (!itemsByType[item.type]) itemsByType[item.type] = 0;
      itemsByType[item.type]++;
    });
    return {
      totalItems: this.index.size,
      itemsByType: itemsByType,
    };
  }
}

if (typeof module !== "undefined") module.exports = { SpotlightIndexer: SpotlightIndexer };
