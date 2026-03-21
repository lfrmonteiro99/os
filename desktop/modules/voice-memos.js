/**
 * Voice Memos Manager (Issue #voice-memos)
 * Supports recording, playback, folders, favorites, and memo management.
 */

class VoiceMemos {
  constructor() {
    this.memos = new Map();
    this.recentlyDeleted = new Map();
    this.folders = new Map();
    this.favorites = new Set();
    this.state = 'idle';
    this.currentRecording = null;
    this._nextMemoId = 1;
    this._nextFolderId = 1;
    this._recordingCount = 0;
    this._playbackStates = new Map();
  }

  getState() {
    return this.state;
  }

  startRecording() {
    if (this.state === 'recording' || this.state === 'paused') {
      throw new Error('Already recording');
    }
    this._recordingCount++;
    this.state = 'recording';
    this.currentRecording = {
      id: this._nextMemoId++,
      name: 'New Recording ' + this._recordingCount,
      createdAt: Date.now(),
      folderId: null,
      enhanced: false,
      trimStart: null,
      trimEnd: null,
    };
  }

  stopRecording(duration) {
    if (this.state !== 'recording' && this.state !== 'paused') {
      throw new Error('Not currently recording');
    }
    var memo = Object.assign({}, this.currentRecording, {
      duration: duration,
      originalDuration: duration,
      size: Math.round(duration * 16000),
    });
    this.memos.set(memo.id, memo);
    this._playbackStates.set(memo.id, 'stopped');
    this.state = 'idle';
    this.currentRecording = null;
    return memo;
  }

  pauseRecording() {
    if (this.state !== 'recording') {
      throw new Error('Not currently recording');
    }
    this.state = 'paused';
  }

  resumeRecording() {
    if (this.state !== 'paused') {
      throw new Error('Not currently paused');
    }
    this.state = 'recording';
  }

  getMemo(id) {
    var memo = this.memos.get(id);
    return memo ? Object.assign({}, memo) : null;
  }

  listMemos() {
    var arr = [];
    this.memos.forEach(function (memo) {
      arr.push(Object.assign({}, memo));
    });
    arr.sort(function (a, b) { return b.createdAt - a.createdAt; });
    return arr;
  }

  renameMemo(id, newName) {
    var memo = this.memos.get(id);
    if (!memo) throw new Error('Memo not found');
    memo.name = newName;
  }

  deleteMemo(id) {
    var memo = this.memos.get(id);
    if (!memo) throw new Error('Memo not found');
    this.memos.delete(id);
    this.favorites.delete(id);
    this._playbackStates.delete(id);
    this.recentlyDeleted.set(id, memo);
  }

  getRecentlyDeleted() {
    var arr = [];
    this.recentlyDeleted.forEach(function (memo) {
      arr.push(Object.assign({}, memo));
    });
    return arr;
  }

  restoreMemo(id) {
    var memo = this.recentlyDeleted.get(id);
    if (!memo) throw new Error('Memo not found in recently deleted');
    this.recentlyDeleted.delete(id);
    this.memos.set(id, memo);
    this._playbackStates.set(id, 'stopped');
  }

  permanentDelete(id) {
    if (!this.recentlyDeleted.has(id)) {
      throw new Error('Memo not found in recently deleted');
    }
    this.recentlyDeleted.delete(id);
  }

  toggleFavorite(id) {
    if (!this.memos.has(id)) throw new Error('Memo not found');
    if (this.favorites.has(id)) {
      this.favorites.delete(id);
    } else {
      this.favorites.add(id);
    }
  }

  getFavorites() {
    var self = this;
    var arr = [];
    this.favorites.forEach(function (id) {
      var memo = self.memos.get(id);
      if (memo) arr.push(Object.assign({}, memo));
    });
    return arr;
  }

  searchMemos(query) {
    var lower = query.toLowerCase();
    var arr = [];
    this.memos.forEach(function (memo) {
      if (memo.name.toLowerCase().indexOf(lower) !== -1) {
        arr.push(Object.assign({}, memo));
      }
    });
    return arr;
  }

  getTotalRecordingTime() {
    var total = 0;
    this.memos.forEach(function (memo) {
      total += memo.duration;
    });
    return total;
  }

  trimMemo(id, startTime, endTime) {
    var memo = this.memos.get(id);
    if (!memo) throw new Error('Memo not found');
    if (startTime >= endTime || endTime > memo.originalDuration) {
      throw new Error('Invalid trim points');
    }
    memo.trimStart = startTime;
    memo.trimEnd = endTime;
    memo.duration = endTime - startTime;
  }

  duplicateMemo(id) {
    var memo = this.memos.get(id);
    if (!memo) throw new Error('Memo not found');
    var dup = Object.assign({}, memo, {
      id: this._nextMemoId++,
      name: memo.name + ' (Copy)',
      createdAt: Date.now(),
    });
    this.memos.set(dup.id, dup);
    this._playbackStates.set(dup.id, 'stopped');
    return dup;
  }

  createFolder(name) {
    var folder = {
      id: this._nextFolderId++,
      name: name,
    };
    this.folders.set(folder.id, folder);
    return folder;
  }

  moveToFolder(memoId, folderId) {
    var memo = this.memos.get(memoId);
    if (!memo) throw new Error('Memo not found');
    if (folderId !== null && !this.folders.has(folderId)) {
      throw new Error('Folder not found');
    }
    memo.folderId = folderId;
  }

  listFolders() {
    var arr = [];
    this.folders.forEach(function (folder) {
      arr.push(Object.assign({}, folder));
    });
    return arr;
  }

  sortMemos(by) {
    var memos = this.listMemos();
    if (by === 'name') {
      memos.sort(function (a, b) { return a.name.localeCompare(b.name); });
    } else if (by === 'duration') {
      memos.sort(function (a, b) { return b.duration - a.duration; });
    } else {
      memos.sort(function (a, b) { return b.createdAt - a.createdAt; });
    }
    return memos;
  }

  enhanceRecording(id) {
    var memo = this.memos.get(id);
    if (!memo) throw new Error('Memo not found');
    memo.enhanced = !memo.enhanced;
  }

  exportMemoMetadata(id) {
    var memo = this.memos.get(id);
    if (!memo) throw new Error('Memo not found');
    return {
      id: memo.id,
      name: memo.name,
      duration: memo.duration,
      createdAt: memo.createdAt,
      size: memo.size,
      enhanced: memo.enhanced,
      folderId: memo.folderId,
    };
  }

  getPlaybackState(id) {
    if (!this.memos.has(id)) throw new Error('Memo not found');
    return this._playbackStates.get(id) || 'stopped';
  }

  playMemo(id) {
    if (!this.memos.has(id)) throw new Error('Memo not found');
    this._playbackStates.set(id, 'playing');
  }

  pausePlayback(id) {
    if (!this.memos.has(id)) throw new Error('Memo not found');
    this._playbackStates.set(id, 'paused');
  }

  stopPlayback(id) {
    if (!this.memos.has(id)) throw new Error('Memo not found');
    this._playbackStates.set(id, 'stopped');
  }
}

module.exports = { VoiceMemos };
