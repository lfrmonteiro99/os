/**
 * TDD Tests for Notes Application
 * RED phase: write tests before implementation
 */
const { NotesApp } = require('../modules/notes');

describe('NotesApp', () => {
  let app;

  beforeEach(() => {
    app = new NotesApp();
  });

  describe('constructor', () => {
    test('starts with empty notes', () => {
      expect(app.listNotes()).toEqual([]);
    });

    test('has default "All Notes" folder', () => {
      const folders = app.getFolders();
      expect(folders.find(f => f.name === 'All Notes')).toBeDefined();
    });
  });

  describe('createNote()', () => {
    test('creates a note with title, body, and folder', () => {
      const note = app.createNote({ title: 'My Note', body: 'Hello world' });
      expect(note.title).toBe('My Note');
      expect(note.body).toBe('Hello world');
    });

    test('returns note with id and timestamps', () => {
      const note = app.createNote({ title: 'Test', body: '' });
      expect(note.id).toBeDefined();
      expect(note.createdAt).toBeDefined();
      expect(note.updatedAt).toBeDefined();
    });

    test('assigns unique ids', () => {
      const a = app.createNote({ title: 'A', body: '' });
      const b = app.createNote({ title: 'B', body: '' });
      expect(a.id).not.toBe(b.id);
    });

    test('assigns note to specified folder', () => {
      const folder = app.createFolder('Work');
      const note = app.createNote({ title: 'Task', body: '', folderId: folder.id });
      expect(note.folderId).toBe(folder.id);
    });

    test('defaults to All Notes folder when no folder specified', () => {
      const note = app.createNote({ title: 'X', body: '' });
      const allNotes = app.getFolders().find(f => f.name === 'All Notes');
      expect(note.folderId).toBe(allNotes.id);
    });
  });

  describe('getNote()', () => {
    test('retrieves a note by id', () => {
      const created = app.createNote({ title: 'Find me', body: 'Content' });
      const found = app.getNote(created.id);
      expect(found.title).toBe('Find me');
      expect(found.body).toBe('Content');
    });

    test('returns null for non-existent id', () => {
      expect(app.getNote(999)).toBeNull();
    });
  });

  describe('updateNote()', () => {
    test('updates title and body', () => {
      const note = app.createNote({ title: 'Old', body: 'Old body' });
      app.updateNote(note.id, { title: 'New', body: 'New body' });
      const updated = app.getNote(note.id);
      expect(updated.title).toBe('New');
      expect(updated.body).toBe('New body');
    });

    test('updates the updatedAt timestamp', () => {
      const note = app.createNote({ title: 'X', body: '' });
      const originalUpdatedAt = note.updatedAt;
      app.updateNote(note.id, { title: 'Y' });
      expect(app.getNote(note.id).updatedAt).toBeGreaterThanOrEqual(originalUpdatedAt);
    });

    test('throws for unknown note id', () => {
      expect(() => app.updateNote(999, { title: 'X' })).toThrow('Note not found');
    });
  });

  describe('deleteNote()', () => {
    test('moves note to trash', () => {
      const note = app.createNote({ title: 'Delete me', body: '' });
      app.deleteNote(note.id);
      expect(app.getNote(note.id)).toBeNull();
      expect(app.getTrash().length).toBe(1);
      expect(app.getTrash()[0].title).toBe('Delete me');
    });
  });

  describe('listNotes()', () => {
    test('returns notes sorted by updatedAt descending', () => {
      const a = app.createNote({ title: 'First', body: '' });
      const b = app.createNote({ title: 'Second', body: '' });
      app.updateNote(a.id, { title: 'First Updated' });
      const list = app.listNotes();
      expect(list[0].title).toBe('First Updated');
      expect(list[1].title).toBe('Second');
    });
  });

  describe('Folders', () => {
    test('createFolder creates a new folder', () => {
      const folder = app.createFolder('Personal');
      expect(folder.id).toBeDefined();
      expect(folder.name).toBe('Personal');
    });

    test('renameFolder renames an existing folder', () => {
      const folder = app.createFolder('Old Name');
      app.renameFolder(folder.id, 'New Name');
      const folders = app.getFolders();
      expect(folders.find(f => f.id === folder.id).name).toBe('New Name');
    });

    test('deleteFolder removes folder and moves its notes to All Notes', () => {
      const folder = app.createFolder('Temp');
      const note = app.createNote({ title: 'Orphan', body: '', folderId: folder.id });
      app.deleteFolder(folder.id);
      expect(app.getFolders().find(f => f.id === folder.id)).toBeUndefined();
      const allNotes = app.getFolders().find(f => f.name === 'All Notes');
      expect(app.getNote(note.id).folderId).toBe(allNotes.id);
    });
  });

  describe('moveNoteToFolder()', () => {
    test('moves a note to a different folder', () => {
      const folder = app.createFolder('Work');
      const note = app.createNote({ title: 'Task', body: '' });
      app.moveNoteToFolder(note.id, folder.id);
      expect(app.getNote(note.id).folderId).toBe(folder.id);
    });
  });

  describe('getNotesByFolder()', () => {
    test('returns notes belonging to a specific folder', () => {
      const folder = app.createFolder('Work');
      app.createNote({ title: 'Work Note', body: '', folderId: folder.id });
      app.createNote({ title: 'Other Note', body: '' });
      const notes = app.getNotesByFolder(folder.id);
      expect(notes.length).toBe(1);
      expect(notes[0].title).toBe('Work Note');
    });
  });

  describe('Pin/Unpin', () => {
    test('pinNote makes note pinned', () => {
      const note = app.createNote({ title: 'Pin me', body: '' });
      app.pinNote(note.id);
      expect(app.getNote(note.id).pinned).toBe(true);
    });

    test('unpinNote removes pin', () => {
      const note = app.createNote({ title: 'Pin me', body: '' });
      app.pinNote(note.id);
      app.unpinNote(note.id);
      expect(app.getNote(note.id).pinned).toBe(false);
    });

    test('pinned notes appear first in listNotes', () => {
      app.createNote({ title: 'Regular', body: '' });
      const pinned = app.createNote({ title: 'Pinned', body: '' });
      app.pinNote(pinned.id);
      const list = app.listNotes();
      expect(list[0].title).toBe('Pinned');
    });
  });

  describe('Lock/Unlock', () => {
    test('lockNote locks a note with password', () => {
      const note = app.createNote({ title: 'Secret', body: 'Hidden' });
      app.lockNote(note.id, 'pass123');
      expect(app.getNote(note.id).locked).toBe(true);
    });

    test('unlockNote with correct password unlocks note', () => {
      const note = app.createNote({ title: 'Secret', body: '' });
      app.lockNote(note.id, 'pass123');
      app.unlockNote(note.id, 'pass123');
      expect(app.getNote(note.id).locked).toBe(false);
    });

    test('unlockNote with wrong password throws', () => {
      const note = app.createNote({ title: 'Secret', body: '' });
      app.lockNote(note.id, 'pass123');
      expect(() => app.unlockNote(note.id, 'wrong')).toThrow('Incorrect password');
    });
  });

  describe('Search', () => {
    test('searches notes by title and body content', () => {
      app.createNote({ title: 'Grocery List', body: 'Buy milk and eggs' });
      app.createNote({ title: 'Meeting Notes', body: 'Discuss budget' });
      app.createNote({ title: 'Random', body: 'Nothing relevant' });
      const results = app.searchNotes('milk');
      expect(results.length).toBe(1);
      expect(results[0].title).toBe('Grocery List');
    });

    test('search is case-insensitive', () => {
      app.createNote({ title: 'IMPORTANT', body: '' });
      const results = app.searchNotes('important');
      expect(results.length).toBe(1);
    });
  });

  describe('Tags', () => {
    test('addTag adds a tag to a note', () => {
      const note = app.createNote({ title: 'Tagged', body: '' });
      app.addTag(note.id, 'work');
      expect(app.getNote(note.id).tags).toContain('work');
    });

    test('removeTag removes a tag from a note', () => {
      const note = app.createNote({ title: 'Tagged', body: '' });
      app.addTag(note.id, 'work');
      app.removeTag(note.id, 'work');
      expect(app.getNote(note.id).tags).not.toContain('work');
    });

    test('getByTag returns notes with a specific tag', () => {
      const a = app.createNote({ title: 'A', body: '' });
      const b = app.createNote({ title: 'B', body: '' });
      app.addTag(a.id, 'urgent');
      app.addTag(b.id, 'later');
      const results = app.getByTag('urgent');
      expect(results.length).toBe(1);
      expect(results[0].title).toBe('A');
    });
  });

  describe('Trash', () => {
    test('getTrash returns deleted notes', () => {
      const note = app.createNote({ title: 'Trashed', body: '' });
      app.deleteNote(note.id);
      expect(app.getTrash().length).toBe(1);
    });

    test('restoreFromTrash restores a deleted note', () => {
      const note = app.createNote({ title: 'Restore me', body: '' });
      app.deleteNote(note.id);
      app.restoreFromTrash(note.id);
      expect(app.getNote(note.id)).not.toBeNull();
      expect(app.getNote(note.id).title).toBe('Restore me');
      expect(app.getTrash().length).toBe(0);
    });

    test('emptyTrash permanently removes all trashed notes', () => {
      const a = app.createNote({ title: 'A', body: '' });
      const b = app.createNote({ title: 'B', body: '' });
      app.deleteNote(a.id);
      app.deleteNote(b.id);
      app.emptyTrash();
      expect(app.getTrash().length).toBe(0);
    });
  });

  describe('Character count', () => {
    test('getCharacterCount returns body length', () => {
      const note = app.createNote({ title: 'X', body: 'Hello World' });
      expect(app.getCharacterCount(note.id)).toBe(11);
    });
  });

  describe('Checklist', () => {
    test('addChecklist adds checklist items to a note', () => {
      const note = app.createNote({ title: 'Tasks', body: '' });
      app.addChecklist(note.id, ['Buy milk', 'Walk dog']);
      const updated = app.getNote(note.id);
      expect(updated.checklist.length).toBe(2);
      expect(updated.checklist[0].text).toBe('Buy milk');
      expect(updated.checklist[0].checked).toBe(false);
    });

    test('toggleChecklistItem toggles checked state', () => {
      const note = app.createNote({ title: 'Tasks', body: '' });
      app.addChecklist(note.id, ['Item 1']);
      app.toggleChecklistItem(note.id, 0);
      expect(app.getNote(note.id).checklist[0].checked).toBe(true);
      app.toggleChecklistItem(note.id, 0);
      expect(app.getNote(note.id).checklist[0].checked).toBe(false);
    });
  });

  describe('Attachments', () => {
    test('addAttachment adds an attachment to a note', () => {
      const note = app.createNote({ title: 'With File', body: '' });
      const attachment = app.addAttachment(note.id, { name: 'photo.png', type: 'image/png', size: 1024 });
      expect(attachment.name).toBe('photo.png');
      expect(app.getNote(note.id).attachments.length).toBe(1);
    });
  });

  describe('Sort options', () => {
    test('sortNotes by updatedAt returns notes in descending order', () => {
      const notes = app.listNotes();
      const sorted = app.sortNotes(notes, 'updatedAt');
      for (let i = 1; i < sorted.length; i++) {
        expect(sorted[i - 1].updatedAt).toBeGreaterThanOrEqual(sorted[i].updatedAt);
      }
    });

    test('sortNotes by createdAt', () => {
      const a = app.createNote({ title: 'First', body: '' });
      const b = app.createNote({ title: 'Second', body: '' });
      const sorted = app.sortNotes(app.listNotes(), 'createdAt');
      expect(sorted[0].id).toBe(b.id);
      expect(sorted[1].id).toBe(a.id);
    });

    test('sortNotes by title alphabetically', () => {
      app.createNote({ title: 'Banana', body: '' });
      app.createNote({ title: 'Apple', body: '' });
      const sorted = app.sortNotes(app.listNotes(), 'title');
      expect(sorted[0].title).toBe('Apple');
      expect(sorted[1].title).toBe('Banana');
    });
  });

  describe('duplicateNote()', () => {
    test('creates a copy of an existing note', () => {
      const original = app.createNote({ title: 'Original', body: 'Content' });
      app.addTag(original.id, 'important');
      const copy = app.duplicateNote(original.id);
      expect(copy.id).not.toBe(original.id);
      expect(copy.title).toBe('Original');
      expect(copy.body).toBe('Content');
      expect(copy.tags).toContain('important');
      expect(app.listNotes().length).toBe(2);
    });
  });

  describe('exportAsText()', () => {
    test('exports note as formatted text string', () => {
      const note = app.createNote({ title: 'My Note', body: 'Some content here' });
      const text = app.exportAsText(note.id);
      expect(text).toContain('My Note');
      expect(text).toContain('Some content here');
    });
  });

  describe('Default folders', () => {
    test('has "All Notes" folder by default', () => {
      const folders = app.getFolders();
      expect(folders.find(f => f.name === 'All Notes')).toBeDefined();
    });
  });

  describe('shareNote()', () => {
    test('returns shareable data for a note', () => {
      const note = app.createNote({ title: 'Share me', body: 'Public content' });
      const shared = app.shareNote(note.id);
      expect(shared.title).toBe('Share me');
      expect(shared.body).toBe('Public content');
      expect(shared.sharedAt).toBeDefined();
    });
  });
});
