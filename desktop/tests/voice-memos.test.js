/**
 * TDD Tests for Voice Memos (Issue #voice-memos)
 * RED phase: write tests before implementation
 */
const { VoiceMemos } = require('../modules/voice-memos');

describe('VoiceMemos', () => {
  let vm;

  beforeEach(() => {
    vm = new VoiceMemos();
  });

  describe('constructor', () => {
    test('initializes with empty recordings', () => {
      expect(vm.listMemos()).toEqual([]);
    });

    test('initializes with idle state', () => {
      expect(vm.getState()).toBe('idle');
    });

    test('initializes with no folders', () => {
      expect(vm.listFolders()).toEqual([]);
    });

    test('initializes with no favorites', () => {
      expect(vm.getFavorites()).toEqual([]);
    });
  });

  describe('startRecording()', () => {
    test('sets state to recording', () => {
      vm.startRecording();
      expect(vm.getState()).toBe('recording');
    });

    test('creates a new memo with default name', () => {
      vm.startRecording();
      expect(vm.currentRecording).not.toBeNull();
      expect(vm.currentRecording.name).toBe('New Recording 1');
    });

    test('increments default name counter', () => {
      vm.startRecording();
      vm.stopRecording(5);
      vm.startRecording();
      expect(vm.currentRecording.name).toBe('New Recording 2');
    });

    test('cannot start recording while already recording', () => {
      vm.startRecording();
      expect(() => vm.startRecording()).toThrow('Already recording');
    });

    test('cannot start recording while paused', () => {
      vm.startRecording();
      vm.pauseRecording();
      expect(() => vm.startRecording()).toThrow('Already recording');
    });
  });

  describe('stopRecording()', () => {
    test('sets state to idle', () => {
      vm.startRecording();
      vm.stopRecording(10);
      expect(vm.getState()).toBe('idle');
    });

    test('finalizes memo with duration', () => {
      vm.startRecording();
      const memo = vm.stopRecording(30);
      expect(memo.duration).toBe(30);
    });

    test('adds memo to list', () => {
      vm.startRecording();
      vm.stopRecording(15);
      expect(vm.listMemos().length).toBe(1);
    });

    test('throws if not recording', () => {
      expect(() => vm.stopRecording(5)).toThrow('Not currently recording');
    });

    test('clears currentRecording', () => {
      vm.startRecording();
      vm.stopRecording(10);
      expect(vm.currentRecording).toBeNull();
    });

    test('assigns size based on duration', () => {
      vm.startRecording();
      const memo = vm.stopRecording(60);
      expect(memo.size).toBeGreaterThan(0);
    });
  });

  describe('pauseRecording() / resumeRecording()', () => {
    test('pauseRecording sets state to paused', () => {
      vm.startRecording();
      vm.pauseRecording();
      expect(vm.getState()).toBe('paused');
    });

    test('resumeRecording sets state back to recording', () => {
      vm.startRecording();
      vm.pauseRecording();
      vm.resumeRecording();
      expect(vm.getState()).toBe('recording');
    });

    test('cannot pause when not recording', () => {
      expect(() => vm.pauseRecording()).toThrow('Not currently recording');
    });

    test('cannot resume when not paused', () => {
      expect(() => vm.resumeRecording()).toThrow('Not currently paused');
    });

    test('can stop from paused state', () => {
      vm.startRecording();
      vm.pauseRecording();
      const memo = vm.stopRecording(20);
      expect(memo.duration).toBe(20);
      expect(vm.getState()).toBe('idle');
    });
  });

  describe('getMemo(id)', () => {
    test('returns memo with expected fields', () => {
      vm.startRecording();
      const created = vm.stopRecording(45);
      const memo = vm.getMemo(created.id);
      expect(memo).toHaveProperty('id');
      expect(memo).toHaveProperty('name');
      expect(memo).toHaveProperty('duration');
      expect(memo).toHaveProperty('createdAt');
      expect(memo).toHaveProperty('size');
    });

    test('returns null for nonexistent id', () => {
      expect(vm.getMemo(999)).toBeNull();
    });
  });

  describe('listMemos()', () => {
    test('returns memos sorted by createdAt descending', () => {
      vm.startRecording();
      vm.stopRecording(10);
      vm.startRecording();
      vm.stopRecording(20);
      const memos = vm.listMemos();
      expect(memos[0].createdAt >= memos[1].createdAt).toBe(true);
    });
  });

  describe('renameMemo()', () => {
    test('updates memo name', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.renameMemo(memo.id, 'Interview Notes');
      expect(vm.getMemo(memo.id).name).toBe('Interview Notes');
    });

    test('throws for nonexistent memo', () => {
      expect(() => vm.renameMemo(999, 'test')).toThrow('Memo not found');
    });
  });

  describe('deleteMemo()', () => {
    test('removes memo from main list', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.deleteMemo(memo.id);
      expect(vm.listMemos().length).toBe(0);
    });

    test('moves to recently deleted', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.deleteMemo(memo.id);
      expect(vm.getRecentlyDeleted().length).toBe(1);
    });

    test('throws for nonexistent memo', () => {
      expect(() => vm.deleteMemo(999)).toThrow('Memo not found');
    });
  });

  describe('recently deleted', () => {
    test('restoreMemo brings memo back', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.deleteMemo(memo.id);
      vm.restoreMemo(memo.id);
      expect(vm.listMemos().length).toBe(1);
      expect(vm.getRecentlyDeleted().length).toBe(0);
    });

    test('permanentDelete removes completely', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.deleteMemo(memo.id);
      vm.permanentDelete(memo.id);
      expect(vm.getRecentlyDeleted().length).toBe(0);
      expect(vm.getMemo(memo.id)).toBeNull();
    });

    test('restoreMemo throws for nonexistent', () => {
      expect(() => vm.restoreMemo(999)).toThrow('Memo not found in recently deleted');
    });

    test('permanentDelete throws for nonexistent', () => {
      expect(() => vm.permanentDelete(999)).toThrow('Memo not found in recently deleted');
    });
  });

  describe('favorites', () => {
    test('toggleFavorite adds memo to favorites', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.toggleFavorite(memo.id);
      expect(vm.getFavorites().length).toBe(1);
    });

    test('toggleFavorite twice removes from favorites', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.toggleFavorite(memo.id);
      vm.toggleFavorite(memo.id);
      expect(vm.getFavorites().length).toBe(0);
    });

    test('getFavorites returns only favorited memos', () => {
      vm.startRecording();
      const m1 = vm.stopRecording(10);
      vm.startRecording();
      vm.stopRecording(20);
      vm.toggleFavorite(m1.id);
      expect(vm.getFavorites().length).toBe(1);
      expect(vm.getFavorites()[0].id).toBe(m1.id);
    });

    test('toggleFavorite throws for nonexistent memo', () => {
      expect(() => vm.toggleFavorite(999)).toThrow('Memo not found');
    });
  });

  describe('searchMemos()', () => {
    test('finds memos by name substring', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.renameMemo(memo.id, 'Meeting with Bob');
      expect(vm.searchMemos('Bob').length).toBe(1);
    });

    test('returns empty array for no match', () => {
      vm.startRecording();
      vm.stopRecording(10);
      expect(vm.searchMemos('zzz')).toEqual([]);
    });

    test('search is case-insensitive', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.renameMemo(memo.id, 'Important Lecture');
      expect(vm.searchMemos('important').length).toBe(1);
    });
  });

  describe('getTotalRecordingTime()', () => {
    test('sums durations across all memos', () => {
      vm.startRecording();
      vm.stopRecording(30);
      vm.startRecording();
      vm.stopRecording(45);
      expect(vm.getTotalRecordingTime()).toBe(75);
    });

    test('returns 0 with no memos', () => {
      expect(vm.getTotalRecordingTime()).toBe(0);
    });
  });

  describe('trimMemo()', () => {
    test('sets trim points on a memo', () => {
      vm.startRecording();
      const memo = vm.stopRecording(60);
      vm.trimMemo(memo.id, 10, 50);
      const trimmed = vm.getMemo(memo.id);
      expect(trimmed.trimStart).toBe(10);
      expect(trimmed.trimEnd).toBe(50);
      expect(trimmed.duration).toBe(40);
    });

    test('throws if start is after end', () => {
      vm.startRecording();
      const memo = vm.stopRecording(60);
      expect(() => vm.trimMemo(memo.id, 50, 10)).toThrow('Invalid trim points');
    });

    test('throws if trim exceeds original duration', () => {
      vm.startRecording();
      const memo = vm.stopRecording(30);
      expect(() => vm.trimMemo(memo.id, 0, 60)).toThrow('Invalid trim points');
    });
  });

  describe('duplicateMemo()', () => {
    test('creates a copy with new id', () => {
      vm.startRecording();
      const memo = vm.stopRecording(25);
      const dup = vm.duplicateMemo(memo.id);
      expect(dup.id).not.toBe(memo.id);
      expect(dup.duration).toBe(memo.duration);
    });

    test('appended name includes copy suffix', () => {
      vm.startRecording();
      const memo = vm.stopRecording(25);
      const dup = vm.duplicateMemo(memo.id);
      expect(dup.name).toBe(memo.name + ' (Copy)');
    });

    test('throws for nonexistent memo', () => {
      expect(() => vm.duplicateMemo(999)).toThrow('Memo not found');
    });
  });

  describe('folder organization', () => {
    test('createFolder creates a folder', () => {
      const folder = vm.createFolder('Work');
      expect(folder.name).toBe('Work');
      expect(vm.listFolders().length).toBe(1);
    });

    test('moveToFolder moves memo into folder', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      const folder = vm.createFolder('Personal');
      vm.moveToFolder(memo.id, folder.id);
      expect(vm.getMemo(memo.id).folderId).toBe(folder.id);
    });

    test('moveToFolder with null removes from folder', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      const folder = vm.createFolder('Work');
      vm.moveToFolder(memo.id, folder.id);
      vm.moveToFolder(memo.id, null);
      expect(vm.getMemo(memo.id).folderId).toBeNull();
    });

    test('listFolders returns all folders', () => {
      vm.createFolder('Work');
      vm.createFolder('Personal');
      expect(vm.listFolders().length).toBe(2);
    });

    test('throws for nonexistent folder', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      expect(() => vm.moveToFolder(memo.id, 999)).toThrow('Folder not found');
    });
  });

  describe('recording duration tracking', () => {
    test('stopRecording captures simulated elapsed time', () => {
      vm.startRecording();
      const memo = vm.stopRecording(120);
      expect(memo.duration).toBe(120);
    });
  });

  describe('sortMemos()', () => {
    test('sorts by date descending (default)', () => {
      vm.startRecording();
      vm.stopRecording(10);
      vm.startRecording();
      vm.stopRecording(20);
      const sorted = vm.sortMemos('date');
      expect(sorted[0].createdAt >= sorted[1].createdAt).toBe(true);
    });

    test('sorts by name ascending', () => {
      vm.startRecording();
      const m1 = vm.stopRecording(10);
      vm.renameMemo(m1.id, 'Bravo');
      vm.startRecording();
      const m2 = vm.stopRecording(20);
      vm.renameMemo(m2.id, 'Alpha');
      const sorted = vm.sortMemos('name');
      expect(sorted[0].name).toBe('Alpha');
      expect(sorted[1].name).toBe('Bravo');
    });

    test('sorts by duration descending', () => {
      vm.startRecording();
      vm.stopRecording(10);
      vm.startRecording();
      vm.stopRecording(50);
      const sorted = vm.sortMemos('duration');
      expect(sorted[0].duration).toBeGreaterThanOrEqual(sorted[1].duration);
    });
  });

  describe('enhanceRecording()', () => {
    test('toggles enhance flag on a memo', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.enhanceRecording(memo.id);
      expect(vm.getMemo(memo.id).enhanced).toBe(true);
    });

    test('toggling twice disables enhance', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.enhanceRecording(memo.id);
      vm.enhanceRecording(memo.id);
      expect(vm.getMemo(memo.id).enhanced).toBe(false);
    });
  });

  describe('exportMemoMetadata()', () => {
    test('returns metadata object', () => {
      vm.startRecording();
      const memo = vm.stopRecording(30);
      const exported = vm.exportMemoMetadata(memo.id);
      expect(exported).toHaveProperty('id');
      expect(exported).toHaveProperty('name');
      expect(exported).toHaveProperty('duration');
      expect(exported).toHaveProperty('createdAt');
      expect(exported).toHaveProperty('size');
      expect(exported).toHaveProperty('enhanced');
      expect(exported).toHaveProperty('folderId');
    });

    test('throws for nonexistent memo', () => {
      expect(() => vm.exportMemoMetadata(999)).toThrow('Memo not found');
    });
  });

  describe('playback state tracking', () => {
    test('starts in stopped playback state', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      expect(vm.getPlaybackState(memo.id)).toBe('stopped');
    });

    test('playMemo sets state to playing', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.playMemo(memo.id);
      expect(vm.getPlaybackState(memo.id)).toBe('playing');
    });

    test('pausePlayback sets state to paused', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.playMemo(memo.id);
      vm.pausePlayback(memo.id);
      expect(vm.getPlaybackState(memo.id)).toBe('paused');
    });

    test('stopPlayback sets state to stopped', () => {
      vm.startRecording();
      const memo = vm.stopRecording(10);
      vm.playMemo(memo.id);
      vm.stopPlayback(memo.id);
      expect(vm.getPlaybackState(memo.id)).toBe('stopped');
    });

    test('playMemo throws for nonexistent memo', () => {
      expect(() => vm.playMemo(999)).toThrow('Memo not found');
    });
  });
});
