/**
 * Notes Application
 * Note management with folders, tags, trash, checklists, and sharing.
 */

class NotesApp {
  constructor() {
    this.notes = new Map();
    this.folders = new Map();
    this.trash = new Map();
    this._nextNoteId = 1;
    this._nextFolderId = 1;
    this.createFolder('All Notes');
  }

  // --- Folders ---
  getFolders() {
    var folders = [];
    this.folders.forEach(function (folder) {
      folders.push(Object.assign({}, folder));
    });
    return folders;
  }

  createFolder(name) {
    var folder = { id: this._nextFolderId++, name: name };
    this.folders.set(folder.id, folder);
    return Object.assign({}, folder);
  }

  renameFolder(id, newName) {
    var folder = this.folders.get(id);
    if (!folder) throw new Error('Folder not found');
    folder.name = newName;
  }

  deleteFolder(id) {
    var self = this;
    var allNotesFolder = null;
    this.folders.forEach(function (f) {
      if (f.name === 'All Notes') allNotesFolder = f;
    });
    this.notes.forEach(function (note) {
      if (note.folderId === id) {
        note.folderId = allNotesFolder.id;
      }
    });
    this.folders.delete(id);
  }

  // --- Notes ---
  createNote(opts) {
    var allNotesFolder = null;
    this.folders.forEach(function (f) {
      if (f.name === 'All Notes') allNotesFolder = f;
    });
    var now = Date.now();
    var note = {
      id: this._nextNoteId++,
      title: opts.title || '',
      body: opts.body || '',
      folderId: opts.folderId || allNotesFolder.id,
      pinned: false,
      locked: false,
      password: null,
      tags: [],
      checklist: [],
      attachments: [],
      createdAt: now,
      updatedAt: now,
    };
    this.notes.set(note.id, note);
    return this._copyNote(note);
  }

  getNote(id) {
    var note = this.notes.get(id);
    if (!note) return null;
    return this._copyNote(note);
  }

  updateNote(id, updates) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    var keys = Object.keys(updates);
    for (var i = 0; i < keys.length; i++) {
      var key = keys[i];
      if (key !== 'id' && key !== 'createdAt') {
        note[key] = updates[key];
      }
    }
    note.updatedAt = Date.now();
  }

  deleteNote(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    note.deletedAt = Date.now();
    this.trash.set(id, note);
    this.notes.delete(id);
  }

  listNotes() {
    var result = [];
    this.notes.forEach(function (note) {
      result.push(Object.assign({}, note, { tags: note.tags.slice(), checklist: note.checklist.slice(), attachments: note.attachments.slice() }));
    });
    result.sort(function (a, b) {
      if (a.pinned && !b.pinned) return -1;
      if (!a.pinned && b.pinned) return 1;
      return b.updatedAt - a.updatedAt;
    });
    return result;
  }

  // --- Folder queries ---
  moveNoteToFolder(noteId, folderId) {
    var note = this.notes.get(noteId);
    if (!note) throw new Error('Note not found');
    note.folderId = folderId;
    note.updatedAt = Date.now();
  }

  getNotesByFolder(folderId) {
    var result = [];
    this.notes.forEach(function (note) {
      if (note.folderId === folderId) {
        result.push(Object.assign({}, note, { tags: note.tags.slice() }));
      }
    });
    return result;
  }

  // --- Pin/Unpin ---
  pinNote(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    note.pinned = true;
    note.updatedAt = Date.now();
  }

  unpinNote(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    note.pinned = false;
    note.updatedAt = Date.now();
  }

  // --- Lock/Unlock ---
  lockNote(id, password) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    note.locked = true;
    note.password = password;
  }

  unlockNote(id, password) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    if (note.password !== password) throw new Error('Incorrect password');
    note.locked = false;
    note.password = null;
  }

  // --- Search ---
  searchNotes(query) {
    var lowerQuery = query.toLowerCase();
    var result = [];
    this.notes.forEach(function (note) {
      if (note.title.toLowerCase().indexOf(lowerQuery) !== -1 ||
          note.body.toLowerCase().indexOf(lowerQuery) !== -1) {
        result.push(Object.assign({}, note, { tags: note.tags.slice() }));
      }
    });
    return result;
  }

  // --- Tags ---
  addTag(noteId, tag) {
    var note = this.notes.get(noteId);
    if (!note) throw new Error('Note not found');
    if (note.tags.indexOf(tag) === -1) {
      note.tags.push(tag);
    }
  }

  removeTag(noteId, tag) {
    var note = this.notes.get(noteId);
    if (!note) throw new Error('Note not found');
    note.tags = note.tags.filter(function (t) { return t !== tag; });
  }

  getByTag(tag) {
    var result = [];
    this.notes.forEach(function (note) {
      if (note.tags.indexOf(tag) !== -1) {
        result.push(Object.assign({}, note, { tags: note.tags.slice() }));
      }
    });
    return result;
  }

  // --- Trash ---
  getTrash() {
    var result = [];
    this.trash.forEach(function (note) {
      result.push(Object.assign({}, note, { tags: note.tags.slice() }));
    });
    return result;
  }

  restoreFromTrash(id) {
    var note = this.trash.get(id);
    if (!note) throw new Error('Note not found in trash');
    delete note.deletedAt;
    this.notes.set(id, note);
    this.trash.delete(id);
  }

  emptyTrash() {
    this.trash.clear();
  }

  // --- Character count ---
  getCharacterCount(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    return note.body.length;
  }

  // --- Checklist ---
  addChecklist(noteId, items) {
    var note = this.notes.get(noteId);
    if (!note) throw new Error('Note not found');
    note.checklist = items.map(function (text) {
      return { text: text, checked: false };
    });
    note.updatedAt = Date.now();
  }

  toggleChecklistItem(noteId, index) {
    var note = this.notes.get(noteId);
    if (!note) throw new Error('Note not found');
    if (index < 0 || index >= note.checklist.length) throw new Error('Invalid checklist index');
    note.checklist[index].checked = !note.checklist[index].checked;
    note.updatedAt = Date.now();
  }

  // --- Attachments ---
  addAttachment(noteId, opts) {
    var note = this.notes.get(noteId);
    if (!note) throw new Error('Note not found');
    var attachment = {
      name: opts.name,
      type: opts.type,
      size: opts.size,
      addedAt: Date.now(),
    };
    note.attachments.push(attachment);
    note.updatedAt = Date.now();
    return Object.assign({}, attachment);
  }

  // --- Sort ---
  sortNotes(notes, by) {
    var arr = notes.slice();
    if (by === 'updatedAt') {
      arr.sort(function (a, b) { return b.updatedAt - a.updatedAt || b.id - a.id; });
    } else if (by === 'createdAt') {
      arr.sort(function (a, b) { return b.createdAt - a.createdAt || b.id - a.id; });
    } else if (by === 'title') {
      arr.sort(function (a, b) { return a.title.localeCompare(b.title); });
    }
    return arr;
  }

  // --- Duplicate ---
  duplicateNote(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    var copy = this.createNote({
      title: note.title,
      body: note.body,
      folderId: note.folderId,
    });
    var copyNote = this.notes.get(copy.id);
    copyNote.tags = note.tags.slice();
    copyNote.checklist = note.checklist.map(function (item) {
      return { text: item.text, checked: item.checked };
    });
    return this._copyNote(copyNote);
  }

  // --- Export ---
  exportAsText(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    return note.title + '\n\n' + note.body;
  }

  // --- Share ---
  shareNote(id) {
    var note = this.notes.get(id);
    if (!note) throw new Error('Note not found');
    return {
      title: note.title,
      body: note.body,
      tags: note.tags.slice(),
      sharedAt: Date.now(),
    };
  }

  // --- Internal ---
  _copyNote(note) {
    return Object.assign({}, note, {
      tags: note.tags.slice(),
      checklist: note.checklist.map(function (item) {
        return Object.assign({}, item);
      }),
      attachments: note.attachments.map(function (att) {
        return Object.assign({}, att);
      }),
    });
  }
}

module.exports = { NotesApp };
