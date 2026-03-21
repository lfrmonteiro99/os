/**
 * Reminders / To-Do Application (Issue #29)
 * Task management with lists, smart filters, sorting, and flags.
 */

class RemindersApp {
  constructor() {
    this._lists = [{ id: 1, name: 'Reminders', color: '#007aff' }];
    this._tasks = [];
    this._nextListId = 2;
    this._nextTaskId = 1;
  }

  // --- Lists ---
  getLists() {
    return this._lists.slice();
  }

  addList(name, color) {
    if (this._lists.some(function (l) { return l.name === name; })) {
      throw new Error('List already exists');
    }
    var list = { id: this._nextListId++, name: name, color: color || '#007aff' };
    this._lists.push(list);
    return list;
  }

  removeList(id) {
    this._lists = this._lists.filter(function (l) { return l.id !== id; });
    this._tasks = this._tasks.filter(function (t) { return t.listId !== id; });
  }

  // --- Tasks ---
  getAllTasks() {
    return this._tasks.slice();
  }

  getTask(id) {
    return this._tasks.find(function (t) { return t.id === id; }) || null;
  }

  addTask(opts) {
    if (!opts || !opts.title) throw new Error('Task title is required');
    var task = {
      id: this._nextTaskId++,
      title: opts.title,
      notes: opts.notes || '',
      dueDate: opts.dueDate || null,
      completed: false,
      completedAt: null,
      flagged: opts.flagged || false,
      priority: opts.priority || 'none',
      listId: opts.listId || this._lists[0].id,
      createdAt: Date.now(),
    };
    this._tasks.push(task);
    return task;
  }

  updateTask(id, updates) {
    var task = this.getTask(id);
    if (!task) throw new Error('Task not found');
    Object.keys(updates).forEach(function (key) {
      if (key !== 'id' && key !== 'createdAt') {
        task[key] = updates[key];
      }
    });
  }

  completeTask(id) {
    var task = this.getTask(id);
    if (task) {
      task.completed = true;
      task.completedAt = Date.now();
    }
  }

  uncompleteTask(id) {
    var task = this.getTask(id);
    if (task) {
      task.completed = false;
      task.completedAt = null;
    }
  }

  deleteTask(id) {
    this._tasks = this._tasks.filter(function (t) { return t.id !== id; });
  }

  toggleFlag(id) {
    var task = this.getTask(id);
    if (task) task.flagged = !task.flagged;
  }

  getTasksByList(listId) {
    return this._tasks.filter(function (t) { return t.listId === listId; });
  }

  // --- Smart Lists ---
  getToday() {
    var today = new Date().toISOString().split('T')[0];
    return this._tasks.filter(function (t) { return t.dueDate === today && !t.completed; });
  }

  getScheduled() {
    return this._tasks.filter(function (t) { return t.dueDate && !t.completed; });
  }

  getFlagged() {
    return this._tasks.filter(function (t) { return t.flagged && !t.completed; });
  }

  getCompleted() {
    return this._tasks.filter(function (t) { return t.completed; });
  }

  // --- Sorting ---
  sortTasks(tasks, by) {
    var arr = tasks.slice();

    if (by === 'priority') {
      var pOrder = { high: 0, medium: 1, low: 2, none: 3 };
      arr.sort(function (a, b) {
        var pa = pOrder[a.priority] !== undefined ? pOrder[a.priority] : 3;
        var pb = pOrder[b.priority] !== undefined ? pOrder[b.priority] : 3;
        return pa - pb;
      });
    } else if (by === 'dueDate') {
      arr.sort(function (a, b) {
        if (!a.dueDate) return 1;
        if (!b.dueDate) return -1;
        return a.dueDate.localeCompare(b.dueDate);
      });
    } else if (by === 'title') {
      arr.sort(function (a, b) { return a.title.localeCompare(b.title); });
    }
    return arr;
  }
}

module.exports = { RemindersApp };
