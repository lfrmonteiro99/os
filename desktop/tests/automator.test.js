const { Automator } = require("../modules/automator");

describe("Automator", () => {
  let auto;
  beforeEach(() => {
    auto = new Automator();
    // Register sample actions
    auto.registerAction({ id: "uppercase", name: "Uppercase", category: "Text",
      handler: (params, input) => ({ text: (input.text || params.text || "").toUpperCase() })
    });
    auto.registerAction({ id: "prefix", name: "Add Prefix", category: "Text",
      handler: (params, input) => ({ text: (params.prefix || "") + (input.text || "") })
    });
    auto.registerAction({ id: "count", name: "Count Chars", category: "Text",
      handler: (params, input) => ({ count: (input.text || "").length })
    });
  });

  /* ── Action Library ─────────── */
  describe("Actions", () => {
    test("registerAction stores action", () => {
      expect(auto.getAction("uppercase")).not.toBeNull();
      expect(auto.getAction("uppercase").name).toBe("Uppercase");
    });

    test("getAllActions returns all registered", () => {
      expect(auto.getAllActions()).toHaveLength(3);
    });

    test("getActionsByCategory filters", () => {
      auto.registerAction({ id: "resize", name: "Resize", category: "Image" });
      expect(auto.getActionsByCategory("Text")).toHaveLength(3);
      expect(auto.getActionsByCategory("Image")).toHaveLength(1);
    });

    test("getAction returns null for missing", () => {
      expect(auto.getAction("nope")).toBeNull();
    });
  });

  /* ── Workflow CRUD ──────────── */
  describe("Workflows", () => {
    test("createWorkflow creates with defaults", () => {
      const wf = auto.createWorkflow({ name: "My Flow" });
      expect(wf.name).toBe("My Flow");
      expect(wf.steps).toEqual([]);
      expect(wf.enabled).toBe(true);
      expect(wf.runCount).toBe(0);
    });

    test("getWorkflow retrieves by id", () => {
      const wf = auto.createWorkflow({ name: "Test" });
      expect(auto.getWorkflow(wf.id).name).toBe("Test");
    });

    test("getAllWorkflows returns all", () => {
      auto.createWorkflow({ name: "A" });
      auto.createWorkflow({ name: "B" });
      expect(auto.getAllWorkflows()).toHaveLength(2);
    });

    test("deleteWorkflow removes", () => {
      const wf = auto.createWorkflow({ name: "X" });
      expect(auto.deleteWorkflow(wf.id)).toBe(true);
      expect(auto.getAllWorkflows()).toHaveLength(0);
    });

    test("duplicateWorkflow creates copy", () => {
      const wf = auto.createWorkflow({ name: "Original" });
      auto.addStep(wf.id, "uppercase", {});
      const dup = auto.duplicateWorkflow(wf.id);
      expect(dup.name).toBe("Original Copy");
      expect(dup.steps).toHaveLength(1);
      expect(dup.id).not.toBe(wf.id);
    });
  });

  /* ── Steps ──────────────────── */
  describe("Workflow Steps", () => {
    test("addStep adds action step", () => {
      const wf = auto.createWorkflow({ name: "Flow" });
      const step = auto.addStep(wf.id, "uppercase", {});
      expect(step.actionId).toBe("uppercase");
      expect(step.id).toBe(1);
    });

    test("addStep returns null for missing action", () => {
      const wf = auto.createWorkflow({ name: "Flow" });
      expect(auto.addStep(wf.id, "bogus", {})).toBeNull();
    });

    test("removeStep removes by index", () => {
      const wf = auto.createWorkflow({ name: "Flow" });
      auto.addStep(wf.id, "uppercase", {});
      auto.addStep(wf.id, "prefix", { prefix: "!" });
      expect(auto.removeStep(wf.id, 0)).toBe(true);
      expect(auto.getWorkflow(wf.id).steps).toHaveLength(1);
      expect(auto.getWorkflow(wf.id).steps[0].id).toBe(1); // renumbered
    });

    test("reorderStep moves step position", () => {
      const wf = auto.createWorkflow({ name: "Flow" });
      auto.addStep(wf.id, "uppercase", {});
      auto.addStep(wf.id, "prefix", { prefix: "!" });
      auto.addStep(wf.id, "count", {});
      expect(auto.reorderStep(wf.id, 0, 2)).toBe(true);
      expect(auto.getWorkflow(wf.id).steps[2].actionId).toBe("uppercase");
    });
  });

  /* ── Execution ──────────────── */
  describe("Workflow Execution", () => {
    test("runWorkflow executes steps in order", () => {
      const wf = auto.createWorkflow({ name: "Pipeline" });
      auto.addStep(wf.id, "uppercase", {});
      const result = auto.runWorkflow(wf.id, { text: "hello" });
      expect(result.success).toBe(true);
      expect(result.results[0].result.text).toBe("HELLO");
    });

    test("steps pipe output to next step", () => {
      const wf = auto.createWorkflow({ name: "Chain" });
      auto.addStep(wf.id, "uppercase", {});
      auto.addStep(wf.id, "prefix", { prefix: ">> " });
      const result = auto.runWorkflow(wf.id, { text: "hi" });
      expect(result.results[1].result.text).toBe(">> HI");
    });

    test("runWorkflow returns null if disabled", () => {
      const wf = auto.createWorkflow({ name: "Off", enabled: false });
      expect(auto.runWorkflow(wf.id)).toBeNull();
    });

    test("runWorkflow increments runCount", () => {
      const wf = auto.createWorkflow({ name: "Counter" });
      auto.addStep(wf.id, "uppercase", {});
      auto.runWorkflow(wf.id, { text: "a" });
      auto.runWorkflow(wf.id, { text: "b" });
      expect(auto.getWorkflow(wf.id).runCount).toBe(2);
    });

    test("runWorkflow records errors for missing actions", () => {
      const wf = auto.createWorkflow({ name: "Broken" });
      wf.steps.push({ id: 1, actionId: "deleted", params: {}, enabled: true });
      const result = auto.runWorkflow(wf.id, {});
      expect(result.success).toBe(false);
      expect(result.errors).toHaveLength(1);
    });

    test("disabled steps are skipped", () => {
      const wf = auto.createWorkflow({ name: "Skip" });
      auto.addStep(wf.id, "uppercase", {});
      wf.steps[0].enabled = false;
      const result = auto.runWorkflow(wf.id, { text: "hi" });
      expect(result.results).toHaveLength(0);
    });
  });

  /* ── History ────────────────── */
  describe("Run History", () => {
    test("getRunHistory returns all runs", () => {
      const wf = auto.createWorkflow({ name: "H" });
      auto.addStep(wf.id, "uppercase", {});
      auto.runWorkflow(wf.id, { text: "a" });
      auto.runWorkflow(wf.id, { text: "b" });
      expect(auto.getRunHistory()).toHaveLength(2);
    });

    test("getRunHistory filters by workflowId", () => {
      const wf1 = auto.createWorkflow({ name: "A" });
      const wf2 = auto.createWorkflow({ name: "B" });
      auto.addStep(wf1.id, "uppercase", {});
      auto.addStep(wf2.id, "count", {});
      auto.runWorkflow(wf1.id, { text: "x" });
      auto.runWorkflow(wf2.id, { text: "y" });
      expect(auto.getRunHistory(wf1.id)).toHaveLength(1);
    });
  });

  /* ── Triggers & Enable ──────── */
  describe("Triggers", () => {
    test("setTrigger assigns trigger", () => {
      const wf = auto.createWorkflow({ name: "Sched" });
      auto.setTrigger(wf.id, { type: "schedule", config: { interval: 3600 } });
      expect(auto.getWorkflow(wf.id).trigger.type).toBe("schedule");
    });

    test("getScheduledWorkflows returns only scheduled+enabled", () => {
      const wf1 = auto.createWorkflow({ name: "S1" });
      auto.setTrigger(wf1.id, { type: "schedule" });
      const wf2 = auto.createWorkflow({ name: "S2", enabled: false });
      auto.setTrigger(wf2.id, { type: "schedule" });
      expect(auto.getScheduledWorkflows()).toHaveLength(1);
    });

    test("enable/disable workflow toggles", () => {
      const wf = auto.createWorkflow({ name: "T" });
      auto.disableWorkflow(wf.id);
      expect(auto.getWorkflow(wf.id).enabled).toBe(false);
      auto.enableWorkflow(wf.id);
      expect(auto.getWorkflow(wf.id).enabled).toBe(true);
    });
  });
});
