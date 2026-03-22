/**
 * Calendar Event Management CRUD (Issue #34)
 * Full event lifecycle with calendars, recurrence, and date queries.
 */

class CalendarManager {
  constructor() {
    this._calendars = [{ id: 1, name: 'Personal', color: '#007aff' }];
    this._events = [];
    this._nextCalId = 2;
    this._nextEvId = 1;
  }

  // --- Calendars ---
  getCalendars() {
    return this._calendars.slice();
  }

  addCalendar(name, color) {
    if (this._calendars.some(function (c) { return c.name === name; })) {
      throw new Error('Calendar already exists');
    }
    var cal = { id: this._nextCalId++, name: name, color: color || '#007aff' };
    this._calendars.push(cal);
    return cal;
  }

  removeCalendar(id) {
    this._calendars = this._calendars.filter(function (c) { return c.id !== id; });
    this._events = this._events.filter(function (e) { return e.calendarId !== id; });
  }

  // --- Events ---
  getAllEvents() {
    return this._events.slice();
  }

  eventCount() {
    return this._events.length;
  }

  getEvent(id) {
    return this._events.find(function (e) { return e.id === id; }) || null;
  }

  addEvent(opts) {
    if (!opts || !opts.title) throw new Error('Event title is required');
    if (!opts.date) throw new Error('Event date is required');

    var allDay = !opts.startTime && !opts.endTime;
    var calId = opts.calendarId || this._calendars[0].id;
    var ev = {
      id: this._nextEvId++,
      title: opts.title,
      date: opts.date,
      startTime: opts.startTime || null,
      endTime: opts.endTime || null,
      allDay: allDay,
      location: opts.location || '',
      notes: opts.notes || '',
      color: opts.color || '',
      calendarId: calId,
      recurrence: opts.recurrence || 'none',
      createdAt: Date.now(),
    };
    this._events.push(ev);
    return ev;
  }

  updateEvent(id, updates) {
    var ev = this.getEvent(id);
    if (!ev) throw new Error('Event not found');
    Object.keys(updates).forEach(function (key) {
      if (key !== 'id' && key !== 'createdAt') {
        ev[key] = updates[key];
      }
    });
    // Recalculate allDay
    if (updates.startTime !== undefined || updates.endTime !== undefined) {
      ev.allDay = !ev.startTime && !ev.endTime;
    }
  }

  deleteEvent(id) {
    this._events = this._events.filter(function (e) { return e.id !== id; });
  }

  getEventsForDate(date) {
    return this._events.filter(function (e) { return e.date === date; });
  }

  getEventsForMonth(year, month) {
    var prefix = year + '-' + String(month).padStart(2, '0');
    return this._events.filter(function (e) { return e.date.startsWith(prefix); });
  }

  getRecurringEvents() {
    return this._events.filter(function (e) { return e.recurrence !== 'none'; });
  }

  getDatesWithEvents(year, month) {
    var events = this.getEventsForMonth(year, month);
    var dates = {};
    events.forEach(function (e) { dates[e.date] = true; });
    return Object.keys(dates);
  }
}

module.exports = { CalendarManager };
