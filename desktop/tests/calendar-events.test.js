/**
 * TDD Tests for Calendar Event Management CRUD (Issue #34)
 * RED phase: write tests before implementation
 */
const { CalendarManager } = require('../modules/calendar-events');

describe('CalendarManager', () => {
  let cal;

  beforeEach(() => {
    cal = new CalendarManager();
  });

  describe('constructor', () => {
    test('starts with no events', () => {
      expect(cal.getAllEvents()).toEqual([]);
      expect(cal.eventCount()).toBe(0);
    });

    test('has a default calendar', () => {
      expect(cal.getCalendars().length).toBeGreaterThan(0);
      expect(cal.getCalendars()[0].name).toBe('Personal');
    });
  });

  describe('addCalendar()', () => {
    test('adds a named calendar with color', () => {
      cal.addCalendar('Work', '#ff3b30');
      const calendars = cal.getCalendars();
      expect(calendars.find(c => c.name === 'Work')).toBeDefined();
    });

    test('rejects duplicate calendar name', () => {
      expect(() => cal.addCalendar('Personal', '#000')).toThrow('Calendar already exists');
    });
  });

  describe('removeCalendar()', () => {
    test('removes calendar and its events', () => {
      cal.addCalendar('Temp', '#ccc');
      const calId = cal.getCalendars().find(c => c.name === 'Temp').id;
      cal.addEvent({ title: 'X', date: '2026-03-21', calendarId: calId });
      cal.removeCalendar(calId);
      expect(cal.getCalendars().find(c => c.name === 'Temp')).toBeUndefined();
      expect(cal.eventCount()).toBe(0);
    });
  });

  describe('addEvent()', () => {
    test('creates an event with required fields', () => {
      cal.addEvent({ title: 'Team Meeting', date: '2026-03-21', startTime: '10:00', endTime: '11:00' });
      expect(cal.eventCount()).toBe(1);
      const ev = cal.getAllEvents()[0];
      expect(ev.title).toBe('Team Meeting');
      expect(ev.date).toBe('2026-03-21');
    });

    test('assigns unique id', () => {
      cal.addEvent({ title: 'A', date: '2026-03-21' });
      cal.addEvent({ title: 'B', date: '2026-03-21' });
      const ids = cal.getAllEvents().map(e => e.id);
      expect(new Set(ids).size).toBe(2);
    });

    test('defaults to all-day if no time specified', () => {
      cal.addEvent({ title: 'Holiday', date: '2026-12-25' });
      expect(cal.getAllEvents()[0].allDay).toBe(true);
    });

    test('sets allDay false when times provided', () => {
      cal.addEvent({ title: 'Meeting', date: '2026-03-21', startTime: '14:00', endTime: '15:00' });
      expect(cal.getAllEvents()[0].allDay).toBe(false);
    });

    test('assigns to default calendar if none specified', () => {
      cal.addEvent({ title: 'Test', date: '2026-03-21' });
      const defaultCal = cal.getCalendars()[0];
      expect(cal.getAllEvents()[0].calendarId).toBe(defaultCal.id);
    });

    test('supports optional fields: location, notes, color', () => {
      cal.addEvent({ title: 'Lunch', date: '2026-03-21', location: 'Cafe', notes: 'Bring laptop', color: '#34c759' });
      const ev = cal.getAllEvents()[0];
      expect(ev.location).toBe('Cafe');
      expect(ev.notes).toBe('Bring laptop');
    });

    test('rejects event without title', () => {
      expect(() => cal.addEvent({ date: '2026-03-21' })).toThrow('Event title is required');
    });

    test('rejects event without date', () => {
      expect(() => cal.addEvent({ title: 'X' })).toThrow('Event date is required');
    });
  });

  describe('updateEvent()', () => {
    test('updates event fields', () => {
      cal.addEvent({ title: 'Old', date: '2026-03-21' });
      const id = cal.getAllEvents()[0].id;
      cal.updateEvent(id, { title: 'New', location: 'Office' });
      expect(cal.getEvent(id).title).toBe('New');
      expect(cal.getEvent(id).location).toBe('Office');
    });

    test('throws for unknown event id', () => {
      expect(() => cal.updateEvent(999, { title: 'X' })).toThrow('Event not found');
    });
  });

  describe('deleteEvent()', () => {
    test('removes event by id', () => {
      cal.addEvent({ title: 'A', date: '2026-03-21' });
      const id = cal.getAllEvents()[0].id;
      cal.deleteEvent(id);
      expect(cal.eventCount()).toBe(0);
    });
  });

  describe('getEventsForDate()', () => {
    test('returns only events on specific date', () => {
      cal.addEvent({ title: 'A', date: '2026-03-21' });
      cal.addEvent({ title: 'B', date: '2026-03-22' });
      cal.addEvent({ title: 'C', date: '2026-03-21' });
      const events = cal.getEventsForDate('2026-03-21');
      expect(events.length).toBe(2);
    });

    test('returns empty for date with no events', () => {
      expect(cal.getEventsForDate('2026-01-01')).toEqual([]);
    });
  });

  describe('getEventsForMonth()', () => {
    test('returns events for given year/month', () => {
      cal.addEvent({ title: 'A', date: '2026-03-05' });
      cal.addEvent({ title: 'B', date: '2026-03-28' });
      cal.addEvent({ title: 'C', date: '2026-04-01' });
      expect(cal.getEventsForMonth(2026, 3).length).toBe(2);
    });
  });

  describe('recurrence', () => {
    test('supports recurrence field', () => {
      cal.addEvent({ title: 'Standup', date: '2026-03-21', recurrence: 'daily' });
      expect(cal.getAllEvents()[0].recurrence).toBe('daily');
    });

    test('defaults recurrence to none', () => {
      cal.addEvent({ title: 'Once', date: '2026-03-21' });
      expect(cal.getAllEvents()[0].recurrence).toBe('none');
    });

    test('getRecurringEvents returns recurring only', () => {
      cal.addEvent({ title: 'Daily', date: '2026-03-21', recurrence: 'daily' });
      cal.addEvent({ title: 'Weekly', date: '2026-03-21', recurrence: 'weekly' });
      cal.addEvent({ title: 'Once', date: '2026-03-21' });
      expect(cal.getRecurringEvents().length).toBe(2);
    });
  });

  describe('getDatesWithEvents()', () => {
    test('returns set of dates that have events in a month', () => {
      cal.addEvent({ title: 'A', date: '2026-03-05' });
      cal.addEvent({ title: 'B', date: '2026-03-15' });
      cal.addEvent({ title: 'C', date: '2026-03-15' });
      const dates = cal.getDatesWithEvents(2026, 3);
      expect(dates).toContain('2026-03-05');
      expect(dates).toContain('2026-03-15');
      expect(dates.length).toBe(2); // unique dates
    });
  });
});
