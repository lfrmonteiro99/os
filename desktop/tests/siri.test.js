const { Siri } = require("../modules/siri");

describe("Siri", () => {
  let siri;
  beforeEach(() => { siri = new Siri(); });

  /* ── Activation ─────────────── */
  describe("Activation", () => {
    test("starts inactive", () => {
      expect(siri.isActive()).toBe(false);
    });

    test("activate/deactivate toggles", () => {
      siri.activate();
      expect(siri.isActive()).toBe(true);
      siri.deactivate();
      expect(siri.isActive()).toBe(false);
    });
  });

  /* ── Query Processing ───────── */
  describe("Queries", () => {
    test("query returns entry with response", () => {
      const r = siri.query("What time is it?");
      expect(r).not.toBeNull();
      expect(r.query).toBe("What time is it?");
      expect(r.response).toBeDefined();
      expect(r.type).toBe("question");
    });

    test("empty query returns null", () => {
      expect(siri.query("")).toBeNull();
      expect(siri.query("  ")).toBeNull();
    });

    test("classifies questions", () => {
      expect(siri.query("What is the weather?").type).toBe("question"); // "What" prefix takes priority
      expect(siri.query("Who is the president?").type).toBe("question");
    });

    test("classifies commands", () => {
      expect(siri.query("Open Safari").type).toBe("command");
      expect(siri.query("Launch Finder").type).toBe("command");
    });

    test("classifies settings", () => {
      expect(siri.query("Set brightness to 50%").type).toBe("setting");
      expect(siri.query("Turn on Do Not Disturb").type).toBe("setting");
    });

    test("classifies reminders", () => {
      expect(siri.query("Remind me to call mom").type).toBe("reminder");
      expect(siri.query("Timer 5 minutes").type).toBe("reminder");
    });

    test("classifies search", () => {
      expect(siri.query("Search for restaurants").type).toBe("search");
    });

    test("classifies media", () => {
      expect(siri.query("Play some music").type).toBe("media");
      expect(siri.query("Skip this song").type).toBe("media");
    });

    test("classifies calculations", () => {
      const r = siri.query("calculate 5 + 3");
      expect(r.type).toBe("calculation");
      expect(r.response.text).toContain("8");
    });

    test("handles division by zero", () => {
      const r = siri.query("10 / 0");
      expect(r.type).toBe("calculation");
    });

    test("time query returns current time", () => {
      const r = siri.query("What time is it?");
      expect(r.response).toBeDefined();
    });
  });

  /* ── Custom Handlers ────────── */
  describe("Handlers", () => {
    test("registerHandler intercepts matching queries", () => {
      siri.registerHandler("weather", () => ({ text: "Sunny, 22°C", action: "weather" }));
      const r = siri.query("What's the weather?");
      expect(r.response.text).toBe("Sunny, 22°C");
    });

    test("removeHandler stops interception", () => {
      siri.registerHandler("hello", () => ({ text: "Hi!" }));
      siri.removeHandler("hello");
      const r = siri.query("hello there");
      expect(r.response.text).not.toBe("Hi!");
    });

    test("handler receives original text", () => {
      let received = null;
      siri.registerHandler("greet", (text) => { received = text; return { text: "ok" }; });
      siri.query("greet me please");
      expect(received).toBe("greet me please");
    });
  });

  /* ── History ────────────────── */
  describe("History", () => {
    test("queries are stored in history", () => {
      siri.query("test 1");
      siri.query("test 2");
      expect(siri.getHistory()).toHaveLength(2);
    });

    test("getLastQuery returns most recent", () => {
      siri.query("first");
      siri.query("second");
      expect(siri.getLastQuery().query).toBe("second");
    });

    test("getLastQuery returns null when empty", () => {
      expect(siri.getLastQuery()).toBeNull();
    });

    test("clearHistory empties history", () => {
      siri.query("a");
      siri.clearHistory();
      expect(siri.getHistory()).toHaveLength(0);
    });

    test("searchHistory filters by query text", () => {
      siri.query("weather today");
      siri.query("open safari");
      siri.query("weather tomorrow");
      expect(siri.searchHistory("weather")).toHaveLength(2);
    });

    test("history limited to maxHistory", () => {
      siri.maxHistory = 3;
      for (let i = 0; i < 5; i++) siri.query("query " + i);
      expect(siri.getHistory()).toHaveLength(3);
    });
  });

  /* ── Suggestions ────────────── */
  describe("Suggestions", () => {
    test("setSuggestions replaces list", () => {
      siri.setSuggestions(["What's the weather?", "Open Music"]);
      expect(siri.getSuggestions()).toHaveLength(2);
    });

    test("addSuggestion appends", () => {
      siri.addSuggestion("Try this");
      expect(siri.getSuggestions()).toContain("Try this");
    });

    test("addSuggestion ignores duplicates", () => {
      siri.addSuggestion("X");
      siri.addSuggestion("X");
      expect(siri.getSuggestions()).toHaveLength(1);
    });

    test("removeSuggestion deletes", () => {
      siri.addSuggestion("A");
      siri.addSuggestion("B");
      siri.removeSuggestion("A");
      expect(siri.getSuggestions()).toEqual(["B"]);
    });

    test("getSuggestions returns empty when disabled", () => {
      siri.addSuggestion("X");
      siri.setPreference("showSuggestions", false);
      expect(siri.getSuggestions()).toEqual([]);
    });
  });

  /* ── Preferences ────────────── */
  describe("Preferences", () => {
    test("setPreference updates valid key", () => {
      expect(siri.setPreference("voice", "samantha")).toBe(true);
      expect(siri.getPreference("voice")).toBe("samantha");
    });

    test("setPreference rejects invalid key", () => {
      expect(siri.setPreference("nonexistent", "x")).toBe(false);
    });

    test("getPreferences returns copy", () => {
      const p = siri.getPreferences();
      expect(p.language).toBe("en-US");
    });
  });

  /* ── Stats ──────────────────── */
  describe("Stats", () => {
    test("getStats returns query breakdown", () => {
      siri.query("What time is it?");
      siri.query("Open Safari");
      siri.query("Who is that?");
      siri.registerHandler("test", () => ({ text: "ok" }));
      const s = siri.getStats();
      expect(s.totalQueries).toBe(3);
      expect(s.handlersRegistered).toBe(1);
      expect(s.byType).toBeDefined();
    });
  });
});
