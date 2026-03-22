const { ActivityMonitor } = require("../modules/activity-monitor");

describe("ActivityMonitor", () => {
  let am;
  beforeEach(() => { am = new ActivityMonitor(); });

  /* ── Process CRUD ───────────── */
  describe("Process Management", () => {
    test("addProcess creates process with defaults", () => {
      const p = am.addProcess({ name: "Finder", cpu: 2.5, memory: 150 });
      expect(p.name).toBe("Finder");
      expect(p.cpu).toBe(2.5);
      expect(p.status).toBe("running");
      expect(p.pid).toBe(1);
    });

    test("getProcess retrieves by pid", () => {
      const p = am.addProcess({ name: "Safari" });
      expect(am.getProcess(p.pid).name).toBe("Safari");
    });

    test("getProcess returns null for missing pid", () => {
      expect(am.getProcess(999)).toBeNull();
    });

    test("getAllProcesses returns all", () => {
      am.addProcess({ name: "A" });
      am.addProcess({ name: "B" });
      am.addProcess({ name: "C" });
      expect(am.getAllProcesses()).toHaveLength(3);
    });

    test("killProcess removes process", () => {
      const p = am.addProcess({ name: "Bad" });
      expect(am.killProcess(p.pid)).toBe(true);
      expect(am.getAllProcesses()).toHaveLength(0);
    });

    test("killProcess returns false for missing", () => {
      expect(am.killProcess(999)).toBe(false);
    });

    test("forceQuit is alias for killProcess", () => {
      const p = am.addProcess({ name: "Frozen" });
      expect(am.forceQuit(p.pid)).toBe(true);
    });

    test("updateProcess modifies fields but not pid", () => {
      const p = am.addProcess({ name: "App", cpu: 5 });
      am.updateProcess(p.pid, { cpu: 15, name: "App2", pid: 999 });
      expect(am.getProcess(p.pid).cpu).toBe(15);
      expect(am.getProcess(p.pid).name).toBe("App2");
      expect(am.getProcess(p.pid).pid).toBe(p.pid); // pid unchanged
    });
  });

  /* ── CPU & Memory ───────────── */
  describe("CPU & Memory", () => {
    test("getCpuUsage sums all process CPU", () => {
      am.addProcess({ name: "A", cpu: 10 });
      am.addProcess({ name: "B", cpu: 25.5 });
      expect(am.getCpuUsage()).toBe(35.5);
    });

    test("getMemoryUsage calculates used/free", () => {
      am.addProcess({ name: "A", memory: 1024 });
      am.addProcess({ name: "B", memory: 2048 });
      const m = am.getMemoryUsage();
      expect(m.used).toBe(3072);
      expect(m.free).toBe(am.memoryTotal - 3072);
      expect(m.total).toBe(8192);
    });

    test("snapshotCpu records history", () => {
      am.addProcess({ name: "A", cpu: 42 });
      const snap = am.snapshotCpu();
      expect(snap.usage).toBe(42);
      expect(am.getCpuHistory()).toHaveLength(1);
    });

    test("CPU history limited to 60 entries", () => {
      for (let i = 0; i < 65; i++) am.snapshotCpu();
      expect(am.getCpuHistory()).toHaveLength(60);
    });
  });

  /* ── Filtering ──────────────── */
  describe("Filtering & Sorting", () => {
    test("setFilter filters by name", () => {
      am.addProcess({ name: "Safari", cpu: 10 });
      am.addProcess({ name: "Finder", cpu: 5 });
      am.addProcess({ name: "Safari Helper", cpu: 8 });
      am.setFilter("safari");
      expect(am.getFilteredProcesses()).toHaveLength(2);
    });

    test("getProcessesByKind filters by kind", () => {
      am.addProcess({ name: "A", kind: "user" });
      am.addProcess({ name: "B", kind: "system" });
      am.addProcess({ name: "C", kind: "user" });
      expect(am.getProcessesByKind("system")).toHaveLength(1);
    });

    test("getProcessesByStatus filters by status", () => {
      am.addProcess({ name: "A", status: "running" });
      am.addProcess({ name: "B", status: "sleeping" });
      expect(am.getProcessesByStatus("sleeping")).toHaveLength(1);
    });

    test("setSortField sorts processes", () => {
      am.addProcess({ name: "B", cpu: 5 });
      am.addProcess({ name: "A", cpu: 10 });
      am.setSortField("name", true);
      am.setFilter("");
      const list = am.getFilteredProcesses();
      expect(list[0].name).toBe("A");
    });
  });

  /* ── Top Consumers ──────────── */
  describe("Top Consumers", () => {
    test("getTopCpu returns top N by CPU", () => {
      am.addProcess({ name: "A", cpu: 5 });
      am.addProcess({ name: "B", cpu: 50 });
      am.addProcess({ name: "C", cpu: 20 });
      const top = am.getTopCpu(2);
      expect(top).toHaveLength(2);
      expect(top[0].name).toBe("B");
    });

    test("getTopMemory returns top N by memory", () => {
      am.addProcess({ name: "A", memory: 100 });
      am.addProcess({ name: "B", memory: 500 });
      const top = am.getTopMemory(1);
      expect(top[0].name).toBe("B");
    });
  });

  /* ── Summary ────────────────── */
  describe("Summary", () => {
    test("getSummary counts statuses", () => {
      am.addProcess({ name: "A", status: "running" });
      am.addProcess({ name: "B", status: "sleeping" });
      am.addProcess({ name: "C", status: "zombie" });
      const s = am.getSummary();
      expect(s.total).toBe(3);
      expect(s.running).toBe(1);
      expect(s.sleeping).toBe(1);
      expect(s.zombie).toBe(1);
      expect(s.stopped).toBe(0);
    });
  });
});
