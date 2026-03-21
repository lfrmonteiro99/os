/**
 * TDD Tests for Reminders / To-Do Application (Issue #29)
 * RED phase: write tests before implementation
 */
const { RemindersApp } = require('../modules/reminders');

describe('RemindersApp', () => {
  let app;

  beforeEach(() => {
    app = new RemindersApp();
  });

  describe('constructor', () => {
    test('starts with default lists', () => {
      const lists = app.getLists();
      expect(lists.length).toBeGreaterThan(0);
      expect(lists.find(l => l.name === 'Reminders')).toBeDefined();
    });

    test('starts with no tasks', () => {
      expect(app.getAllTasks()).toEqual([]);
    });
  });

  describe('addList()', () => {
    test('creates a new list', () => {
      app.addList('Shopping', '#ff9500');
      expect(app.getLists().find(l => l.name === 'Shopping')).toBeDefined();
    });

    test('assigns unique id', () => {
      app.addList('A', '#f00');
      app.addList('B', '#0f0');
      const ids = app.getLists().map(l => l.id);
      expect(new Set(ids).size).toBe(ids.length);
    });

    test('rejects duplicate list name', () => {
      expect(() => app.addList('Reminders', '#000')).toThrow('List already exists');
    });
  });

  describe('removeList()', () => {
    test('removes list and its tasks', () => {
      app.addList('Temp', '#ccc');
      const listId = app.getLists().find(l => l.name === 'Temp').id;
      app.addTask({ title: 'X', listId: listId });
      app.removeList(listId);
      expect(app.getLists().find(l => l.name === 'Temp')).toBeUndefined();
      expect(app.getAllTasks().length).toBe(0);
    });
  });

  describe('addTask()', () => {
    test('adds a task to default list', () => {
      app.addTask({ title: 'Buy milk' });
      expect(app.getAllTasks().length).toBe(1);
      expect(app.getAllTasks()[0].title).toBe('Buy milk');
      expect(app.getAllTasks()[0].completed).toBe(false);
    });

    test('assigns unique id', () => {
      app.addTask({ title: 'A' });
      app.addTask({ title: 'B' });
      const ids = app.getAllTasks().map(t => t.id);
      expect(new Set(ids).size).toBe(2);
    });

    test('supports optional fields: notes, dueDate, priority, flagged', () => {
      app.addTask({
        title: 'Report',
        notes: 'Q1 summary',
        dueDate: '2026-03-25',
        priority: 'high',
        flagged: true,
      });
      const task = app.getAllTasks()[0];
      expect(task.notes).toBe('Q1 summary');
      expect(task.dueDate).toBe('2026-03-25');
      expect(task.priority).toBe('high');
      expect(task.flagged).toBe(true);
    });

    test('defaults priority to none', () => {
      app.addTask({ title: 'X' });
      expect(app.getAllTasks()[0].priority).toBe('none');
    });

    test('rejects task without title', () => {
      expect(() => app.addTask({})).toThrow('Task title is required');
    });

    test('assigns to default list when no listId given', () => {
      app.addTask({ title: 'Test' });
      const defaultList = app.getLists()[0];
      expect(app.getAllTasks()[0].listId).toBe(defaultList.id);
    });
  });

  describe('updateTask()', () => {
    test('updates task fields', () => {
      app.addTask({ title: 'Old' });
      const id = app.getAllTasks()[0].id;
      app.updateTask(id, { title: 'New', notes: 'Updated' });
      expect(app.getTask(id).title).toBe('New');
      expect(app.getTask(id).notes).toBe('Updated');
    });

    test('throws for unknown task id', () => {
      expect(() => app.updateTask(999, { title: 'X' })).toThrow('Task not found');
    });
  });

  describe('completeTask() / uncompleteTask()', () => {
    test('marks task as completed', () => {
      app.addTask({ title: 'X' });
      const id = app.getAllTasks()[0].id;
      app.completeTask(id);
      expect(app.getTask(id).completed).toBe(true);
      expect(app.getTask(id).completedAt).toBeDefined();
    });

    test('uncomplete restores task', () => {
      app.addTask({ title: 'X' });
      const id = app.getAllTasks()[0].id;
      app.completeTask(id);
      app.uncompleteTask(id);
      expect(app.getTask(id).completed).toBe(false);
      expect(app.getTask(id).completedAt).toBeNull();
    });
  });

  describe('deleteTask()', () => {
    test('removes task by id', () => {
      app.addTask({ title: 'A' });
      const id = app.getAllTasks()[0].id;
      app.deleteTask(id);
      expect(app.getAllTasks().length).toBe(0);
    });
  });

  describe('getTasksByList()', () => {
    test('returns tasks for a specific list', () => {
      app.addList('Work', '#f00');
      const workId = app.getLists().find(l => l.name === 'Work').id;
      app.addTask({ title: 'A', listId: workId });
      app.addTask({ title: 'B' }); // default list
      expect(app.getTasksByList(workId).length).toBe(1);
      expect(app.getTasksByList(workId)[0].title).toBe('A');
    });
  });

  describe('smart lists', () => {
    test('getToday returns tasks due today', () => {
      const today = new Date().toISOString().split('T')[0];
      app.addTask({ title: 'Today', dueDate: today });
      app.addTask({ title: 'Tomorrow', dueDate: '2099-12-31' });
      expect(app.getToday().length).toBe(1);
      expect(app.getToday()[0].title).toBe('Today');
    });

    test('getScheduled returns tasks with due dates', () => {
      app.addTask({ title: 'Scheduled', dueDate: '2026-04-01' });
      app.addTask({ title: 'No date' });
      expect(app.getScheduled().length).toBe(1);
    });

    test('getFlagged returns flagged tasks', () => {
      app.addTask({ title: 'Flagged', flagged: true });
      app.addTask({ title: 'Normal' });
      expect(app.getFlagged().length).toBe(1);
    });

    test('getCompleted returns completed tasks', () => {
      app.addTask({ title: 'Done' });
      const id = app.getAllTasks()[0].id;
      app.completeTask(id);
      app.addTask({ title: 'Pending' });
      expect(app.getCompleted().length).toBe(1);
    });
  });

  describe('toggleFlag()', () => {
    test('toggles flagged status', () => {
      app.addTask({ title: 'X' });
      const id = app.getAllTasks()[0].id;
      expect(app.getTask(id).flagged).toBe(false);
      app.toggleFlag(id);
      expect(app.getTask(id).flagged).toBe(true);
      app.toggleFlag(id);
      expect(app.getTask(id).flagged).toBe(false);
    });
  });

  describe('sortTasks()', () => {
    test('sorts by priority (high first)', () => {
      app.addTask({ title: 'Low', priority: 'low' });
      app.addTask({ title: 'High', priority: 'high' });
      app.addTask({ title: 'Med', priority: 'medium' });
      const sorted = app.sortTasks(app.getAllTasks(), 'priority');
      expect(sorted[0].title).toBe('High');
      expect(sorted[1].title).toBe('Med');
      expect(sorted[2].title).toBe('Low');
    });

    test('sorts by dueDate (earliest first)', () => {
      app.addTask({ title: 'Later', dueDate: '2026-12-01' });
      app.addTask({ title: 'Sooner', dueDate: '2026-03-01' });
      const sorted = app.sortTasks(app.getAllTasks(), 'dueDate');
      expect(sorted[0].title).toBe('Sooner');
    });

    test('sorts by title alphabetically', () => {
      app.addTask({ title: 'Banana' });
      app.addTask({ title: 'Apple' });
      const sorted = app.sortTasks(app.getAllTasks(), 'title');
      expect(sorted[0].title).toBe('Apple');
    });
  });
});
