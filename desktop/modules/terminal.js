/* ── Terminal Emulator ─────────────────────────────── */
/* macOS Terminal-style command execution with history and basic commands */

class Terminal {
  constructor(opts) {
    opts = opts || {};
    this.cwd = opts.cwd || "/Users/user";
    this.home = opts.home || "/Users/user";
    this.user = opts.user || "user";
    this.hostname = opts.hostname || "AuroraOS";
    this.history = [];
    this.historyIndex = -1;
    this.maxHistory = opts.maxHistory || 500;
    this.output = [];
    this.env = Object.assign({ HOME: this.home, USER: this.user, SHELL: "/bin/zsh", PATH: "/usr/local/bin:/usr/bin:/bin" }, opts.env || {});
    this.aliases = {};
    this.fileSystem = {};  // simple in-memory FS: { "/path": "content" }
    this.running = false;
  }

  /* ── Command Execution ────────── */
  execute(input) {
    input = input.trim();
    if (!input) return { output: "", exitCode: 0 };

    // Add to history
    if (this.history[this.history.length - 1] !== input) {
      this.history.push(input);
      if (this.history.length > this.maxHistory) this.history.shift();
    }
    this.historyIndex = this.history.length;

    // Resolve aliases
    var parts = input.split(/\s+/);
    var cmd = parts[0];
    var args = parts.slice(1);

    if (this.aliases[cmd]) {
      var expanded = this.aliases[cmd].split(/\s+/);
      cmd = expanded[0];
      args = expanded.slice(1).concat(args);
    }

    // Built-in commands
    var builtins = {
      echo: this._echo.bind(this),
      pwd: this._pwd.bind(this),
      cd: this._cd.bind(this),
      ls: this._ls.bind(this),
      cat: this._cat.bind(this),
      mkdir: this._mkdir.bind(this),
      touch: this._touch.bind(this),
      rm: this._rm.bind(this),
      clear: this._clear.bind(this),
      whoami: this._whoami.bind(this),
      hostname: this._hostname.bind(this),
      date: this._date.bind(this),
      env: this._env.bind(this),
      export: this._export.bind(this),
      alias: this._alias.bind(this),
      history: this._history.bind(this),
      help: this._help.bind(this),
    };

    if (builtins[cmd]) {
      var result = builtins[cmd](args);
      // clear already wiped output, don't re-add
      if (cmd !== "clear") {
        this.output.push({ input: input, output: result.output, exitCode: result.exitCode });
      }
      return result;
    }

    var errMsg = "zsh: command not found: " + cmd;
    this.output.push({ input: input, output: errMsg, exitCode: 127 });
    return { output: errMsg, exitCode: 127 };
  }

  /* ── History Navigation ───────── */
  historyUp() {
    if (this.historyIndex > 0) {
      this.historyIndex--;
      return this.history[this.historyIndex];
    }
    return this.history[0] || "";
  }

  historyDown() {
    if (this.historyIndex < this.history.length - 1) {
      this.historyIndex++;
      return this.history[this.historyIndex];
    }
    this.historyIndex = this.history.length;
    return "";
  }

  getHistory() {
    return this.history.slice();
  }

  clearHistory() {
    this.history = [];
    this.historyIndex = -1;
  }

  /* ── Prompt ───────────────────── */
  getPrompt() {
    var dir = this.cwd === this.home ? "~" : this.cwd.split("/").pop() || "/";
    return this.user + "@" + this.hostname + " " + dir + " % ";
  }

  getOutput() {
    return this.output.slice();
  }

  clearOutput() {
    this.output = [];
  }

  /* ── Path Resolution ──────────── */
  _resolve(path) {
    if (!path) return this.cwd;
    if (path === "~") return this.home;
    if (path.startsWith("~/")) return this.home + path.slice(1);
    if (path.startsWith("/")) return path;
    // Relative path
    var base = this.cwd === "/" ? "" : this.cwd;
    return base + "/" + path;
  }

  /* ── Built-in Commands ────────── */
  _echo(args) {
    return { output: args.join(" "), exitCode: 0 };
  }

  _pwd() {
    return { output: this.cwd, exitCode: 0 };
  }

  _cd(args) {
    var target = args[0] || this.home;
    var resolved = this._resolve(target);
    // Normalize: remove trailing slash, handle ..
    var parts = resolved.split("/").filter(Boolean);
    var stack = [];
    parts.forEach(function (p) {
      if (p === "..") stack.pop();
      else if (p !== ".") stack.push(p);
    });
    this.cwd = "/" + stack.join("/");
    return { output: "", exitCode: 0 };
  }

  _ls(args) {
    var dir = this._resolve(args[0]) || this.cwd;
    var entries = [];
    var prefix = dir === "/" ? "/" : dir + "/";
    var self = this;
    Object.keys(this.fileSystem).forEach(function (path) {
      if (path.startsWith(prefix) || (dir === "/" && path.startsWith("/"))) {
        var rest = path.slice(prefix.length);
        var name = rest.split("/")[0];
        if (name && entries.indexOf(name) === -1) entries.push(name);
      }
    });
    return { output: entries.sort().join("  "), exitCode: 0 };
  }

  _cat(args) {
    if (!args[0]) return { output: "cat: missing operand", exitCode: 1 };
    var path = this._resolve(args[0]);
    if (this.fileSystem[path] !== undefined) {
      return { output: this.fileSystem[path], exitCode: 0 };
    }
    return { output: "cat: " + args[0] + ": No such file or directory", exitCode: 1 };
  }

  _mkdir(args) {
    if (!args[0]) return { output: "mkdir: missing operand", exitCode: 1 };
    var path = this._resolve(args[0]);
    this.fileSystem[path] = null; // null = directory marker
    return { output: "", exitCode: 0 };
  }

  _touch(args) {
    if (!args[0]) return { output: "touch: missing operand", exitCode: 1 };
    var path = this._resolve(args[0]);
    if (this.fileSystem[path] === undefined) this.fileSystem[path] = "";
    return { output: "", exitCode: 0 };
  }

  _rm(args) {
    if (!args[0]) return { output: "rm: missing operand", exitCode: 1 };
    var path = this._resolve(args[0]);
    if (this.fileSystem[path] !== undefined) {
      delete this.fileSystem[path];
      return { output: "", exitCode: 0 };
    }
    return { output: "rm: " + args[0] + ": No such file or directory", exitCode: 1 };
  }

  _clear() {
    this.output = [];
    return { output: "", exitCode: 0 };
  }

  _whoami() {
    return { output: this.user, exitCode: 0 };
  }

  _hostname() {
    return { output: this.hostname, exitCode: 0 };
  }

  _date() {
    return { output: new Date().toString(), exitCode: 0 };
  }

  _env() {
    var lines = [];
    var self = this;
    Object.keys(this.env).sort().forEach(function (k) {
      lines.push(k + "=" + self.env[k]);
    });
    return { output: lines.join("\n"), exitCode: 0 };
  }

  _export(args) {
    if (!args[0]) return this._env();
    var eq = args[0].indexOf("=");
    if (eq === -1) return { output: "export: invalid syntax", exitCode: 1 };
    var key = args[0].slice(0, eq);
    var val = args[0].slice(eq + 1);
    this.env[key] = val;
    return { output: "", exitCode: 0 };
  }

  _alias(args) {
    if (!args[0]) {
      var lines = [];
      var self = this;
      Object.keys(this.aliases).forEach(function (k) {
        lines.push("alias " + k + "='" + self.aliases[k] + "'");
      });
      return { output: lines.join("\n"), exitCode: 0 };
    }
    var eq = args[0].indexOf("=");
    if (eq === -1) return { output: "alias: invalid syntax", exitCode: 1 };
    this.aliases[args[0].slice(0, eq)] = args[0].slice(eq + 1);
    return { output: "", exitCode: 0 };
  }

  _history() {
    var lines = this.history.map(function (h, i) {
      return (i + 1) + "  " + h;
    });
    return { output: lines.join("\n"), exitCode: 0 };
  }

  _help() {
    return { output: "Available commands: echo, pwd, cd, ls, cat, mkdir, touch, rm, clear, whoami, hostname, date, env, export, alias, history, help", exitCode: 0 };
  }
}

if (typeof module !== "undefined") module.exports = { Terminal: Terminal };
