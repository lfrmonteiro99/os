/* ── Activity Monitor ──────────────────────────────── */
/* macOS-style process management with CPU/memory tracking */

class ActivityMonitor {
  constructor() {
    this.processes = [];
    this.nextPid = 1;
    this.cpuHistory = [];        // snapshots over time
    this.memoryTotal = 8 * 1024; // 8 GB in MB
    this.sortField = "cpu";
    this.sortAsc = false;
    this.filter = "";
  }

  /* ── Process Management ───────── */
  addProcess(opts) {
    var proc = {
      pid: this.nextPid++,
      name: opts.name || "Unknown",
      user: opts.user || "root",
      cpu: opts.cpu || 0,           // percentage 0-100
      memory: opts.memory || 0,     // MB
      threads: opts.threads || 1,
      ports: opts.ports || 0,
      status: opts.status || "running",  // running, sleeping, stopped, zombie
      startTime: opts.startTime || Date.now(),
      parentPid: opts.parentPid || 0,
      kind: opts.kind || "user",    // user, system, background
    };
    this.processes.push(proc);
    return proc;
  }

  getProcess(pid) {
    return this.processes.find(function (p) { return p.pid === pid; }) || null;
  }

  getAllProcesses() {
    return this.processes.slice();
  }

  killProcess(pid) {
    var idx = this.processes.findIndex(function (p) { return p.pid === pid; });
    if (idx === -1) return false;
    this.processes.splice(idx, 1);
    return true;
  }

  forceQuit(pid) {
    return this.killProcess(pid);
  }

  /* ── CPU & Memory Stats ───────── */
  getCpuUsage() {
    var total = 0;
    this.processes.forEach(function (p) { total += p.cpu; });
    return Math.round(total * 100) / 100;
  }

  getMemoryUsage() {
    var used = 0;
    this.processes.forEach(function (p) { used += p.memory; });
    return {
      total: this.memoryTotal,
      used: Math.round(used * 100) / 100,
      free: Math.round((this.memoryTotal - used) * 100) / 100,
      usedPercent: Math.round((used / this.memoryTotal) * 10000) / 100,
    };
  }

  snapshotCpu() {
    var snap = { timestamp: Date.now(), usage: this.getCpuUsage() };
    this.cpuHistory.push(snap);
    if (this.cpuHistory.length > 60) this.cpuHistory.shift();
    return snap;
  }

  getCpuHistory() {
    return this.cpuHistory.slice();
  }

  /* ── Process Filtering ────────── */
  setFilter(text) {
    this.filter = text.toLowerCase();
  }

  getFilteredProcesses() {
    var f = this.filter;
    var list = f
      ? this.processes.filter(function (p) {
          return p.name.toLowerCase().indexOf(f) !== -1;
        })
      : this.processes.slice();
    return this._sortList(list);
  }

  getProcessesByKind(kind) {
    return this.processes.filter(function (p) { return p.kind === kind; });
  }

  getProcessesByStatus(status) {
    return this.processes.filter(function (p) { return p.status === status; });
  }

  /* ── Sorting ──────────────────── */
  setSortField(field, asc) {
    this.sortField = field;
    this.sortAsc = asc !== undefined ? asc : false;
  }

  _sortList(list) {
    var field = this.sortField;
    var asc = this.sortAsc;
    return list.sort(function (a, b) {
      var va = a[field], vb = b[field];
      if (typeof va === "string") va = va.toLowerCase();
      if (typeof vb === "string") vb = vb.toLowerCase();
      if (va < vb) return asc ? -1 : 1;
      if (va > vb) return asc ? 1 : -1;
      return 0;
    });
  }

  /* ── Top Consumers ────────────── */
  getTopCpu(n) {
    n = n || 5;
    return this.processes.slice().sort(function (a, b) { return b.cpu - a.cpu; }).slice(0, n);
  }

  getTopMemory(n) {
    n = n || 5;
    return this.processes.slice().sort(function (a, b) { return b.memory - a.memory; }).slice(0, n);
  }

  /* ── System Summary ───────────── */
  getSummary() {
    var running = 0, sleeping = 0, stopped = 0, zombie = 0;
    this.processes.forEach(function (p) {
      if (p.status === "running") running++;
      else if (p.status === "sleeping") sleeping++;
      else if (p.status === "stopped") stopped++;
      else if (p.status === "zombie") zombie++;
    });
    return {
      total: this.processes.length,
      running: running,
      sleeping: sleeping,
      stopped: stopped,
      zombie: zombie,
      cpu: this.getCpuUsage(),
      memory: this.getMemoryUsage(),
    };
  }

  updateProcess(pid, updates) {
    var proc = this.getProcess(pid);
    if (!proc) return null;
    Object.keys(updates).forEach(function (k) {
      if (k !== "pid") proc[k] = updates[k];
    });
    return proc;
  }
}

if (typeof module !== "undefined") module.exports = { ActivityMonitor: ActivityMonitor };
