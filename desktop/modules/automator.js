/* ── Automator / Shortcuts ─────────────────────────── */
/* macOS-style workflow builder with actions and triggers */

class Automator {
  constructor() {
    this.workflows = [];
    this.nextId = 1;
    this.actions = {};      // registered action library
    this.runHistory = [];
    this.maxHistory = 100;
  }

  /* ── Action Library ───────────── */
  registerAction(opts) {
    var action = {
      id: opts.id,
      name: opts.name || opts.id,
      category: opts.category || "General",
      description: opts.description || "",
      inputs: opts.inputs || [],
      handler: opts.handler || function () { return {}; },
    };
    this.actions[action.id] = action;
    return action;
  }

  getAction(id) {
    return this.actions[id] || null;
  }

  getAllActions() {
    return Object.keys(this.actions).map(function (k) { return this.actions[k]; }.bind(this));
  }

  getActionsByCategory(cat) {
    return this.getAllActions().filter(function (a) { return a.category === cat; });
  }

  /* ── Workflow CRUD ────────────── */
  createWorkflow(opts) {
    var wf = {
      id: this.nextId++,
      name: opts.name || "Untitled Workflow",
      description: opts.description || "",
      steps: opts.steps || [],
      trigger: opts.trigger || null,   // { type: "manual"|"schedule"|"event", config: {} }
      enabled: opts.enabled !== false,
      createdAt: Date.now(),
      lastRun: null,
      runCount: 0,
    };
    this.workflows.push(wf);
    return wf;
  }

  getWorkflow(id) {
    return this.workflows.find(function (w) { return w.id === id; }) || null;
  }

  getAllWorkflows() {
    return this.workflows.slice();
  }

  deleteWorkflow(id) {
    var idx = this.workflows.findIndex(function (w) { return w.id === id; });
    if (idx === -1) return false;
    this.workflows.splice(idx, 1);
    return true;
  }

  duplicateWorkflow(id) {
    var wf = this.getWorkflow(id);
    if (!wf) return null;
    return this.createWorkflow({
      name: wf.name + " Copy",
      description: wf.description,
      steps: wf.steps.map(function (s) { return Object.assign({}, s); }),
      trigger: wf.trigger ? Object.assign({}, wf.trigger) : null,
    });
  }

  /* ── Steps ────────────────────── */
  addStep(workflowId, actionId, params) {
    var wf = this.getWorkflow(workflowId);
    var action = this.getAction(actionId);
    if (!wf || !action) return null;
    var step = {
      id: wf.steps.length + 1,
      actionId: actionId,
      params: params || {},
      enabled: true,
    };
    wf.steps.push(step);
    return step;
  }

  removeStep(workflowId, stepIndex) {
    var wf = this.getWorkflow(workflowId);
    if (!wf || stepIndex < 0 || stepIndex >= wf.steps.length) return false;
    wf.steps.splice(stepIndex, 1);
    // Re-number steps
    wf.steps.forEach(function (s, i) { s.id = i + 1; });
    return true;
  }

  reorderStep(workflowId, fromIndex, toIndex) {
    var wf = this.getWorkflow(workflowId);
    if (!wf) return false;
    if (fromIndex < 0 || fromIndex >= wf.steps.length) return false;
    if (toIndex < 0 || toIndex >= wf.steps.length) return false;
    var step = wf.steps.splice(fromIndex, 1)[0];
    wf.steps.splice(toIndex, 0, step);
    wf.steps.forEach(function (s, i) { s.id = i + 1; });
    return true;
  }

  /* ── Execution ────────────────── */
  runWorkflow(id, initialInput) {
    var wf = this.getWorkflow(id);
    if (!wf || !wf.enabled) return null;

    var self = this;
    var context = { input: initialInput || {}, results: [], errors: [] };

    wf.steps.forEach(function (step) {
      if (!step.enabled) return;
      var action = self.getAction(step.actionId);
      if (!action) {
        context.errors.push({ step: step.id, error: "Action not found: " + step.actionId });
        return;
      }
      try {
        var result = action.handler(step.params, context.input);
        context.results.push({ step: step.id, actionId: step.actionId, result: result });
        context.input = result; // pipe output to next step
      } catch (e) {
        context.errors.push({ step: step.id, error: e.message });
      }
    });

    wf.lastRun = Date.now();
    wf.runCount++;

    var record = {
      workflowId: id,
      workflowName: wf.name,
      timestamp: wf.lastRun,
      results: context.results,
      errors: context.errors,
      success: context.errors.length === 0,
    };
    this.runHistory.push(record);
    if (this.runHistory.length > this.maxHistory) this.runHistory.shift();

    return record;
  }

  getRunHistory(workflowId) {
    if (workflowId) {
      return this.runHistory.filter(function (r) { return r.workflowId === workflowId; });
    }
    return this.runHistory.slice();
  }

  /* ── Triggers ─────────────────── */
  setTrigger(workflowId, trigger) {
    var wf = this.getWorkflow(workflowId);
    if (!wf) return false;
    wf.trigger = trigger; // { type, config }
    return true;
  }

  getScheduledWorkflows() {
    return this.workflows.filter(function (w) {
      return w.trigger && w.trigger.type === "schedule" && w.enabled;
    });
  }

  /* ── Enable / Disable ─────────── */
  enableWorkflow(id) {
    var wf = this.getWorkflow(id);
    if (!wf) return false;
    wf.enabled = true;
    return true;
  }

  disableWorkflow(id) {
    var wf = this.getWorkflow(id);
    if (!wf) return false;
    wf.enabled = false;
    return true;
  }
}

if (typeof module !== "undefined") module.exports = { Automator: Automator };
