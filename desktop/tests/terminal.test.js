const { Terminal } = require("../modules/terminal");

describe("Terminal", () => {
  let term;
  beforeEach(() => { term = new Terminal(); });

  /* ── Basic Execution ────────── */
  describe("Command Execution", () => {
    test("empty input returns empty output", () => {
      const r = term.execute("");
      expect(r.output).toBe("");
      expect(r.exitCode).toBe(0);
    });

    test("unknown command returns error 127", () => {
      const r = term.execute("foobar");
      expect(r.exitCode).toBe(127);
      expect(r.output).toContain("command not found");
    });

    test("echo outputs arguments", () => {
      expect(term.execute("echo hello world").output).toBe("hello world");
    });

    test("pwd returns current directory", () => {
      expect(term.execute("pwd").output).toBe("/Users/user");
    });

    test("whoami returns username", () => {
      expect(term.execute("whoami").output).toBe("user");
    });

    test("hostname returns hostname", () => {
      expect(term.execute("hostname").output).toBe("AuroraOS");
    });

    test("date returns a date string", () => {
      const r = term.execute("date");
      expect(r.exitCode).toBe(0);
      expect(r.output.length).toBeGreaterThan(0);
    });

    test("help lists commands", () => {
      const r = term.execute("help");
      expect(r.output).toContain("echo");
      expect(r.output).toContain("cd");
    });
  });

  /* ── Directory Navigation ───── */
  describe("cd", () => {
    test("cd with no args goes home", () => {
      term.execute("cd /tmp");
      term.execute("cd");
      expect(term.execute("pwd").output).toBe("/Users/user");
    });

    test("cd to absolute path", () => {
      term.execute("cd /etc");
      expect(term.execute("pwd").output).toBe("/etc");
    });

    test("cd ~ goes home", () => {
      term.execute("cd /tmp");
      term.execute("cd ~");
      expect(term.execute("pwd").output).toBe("/Users/user");
    });

    test("cd .. goes up one level", () => {
      term.execute("cd /Users/user/Documents");
      term.execute("cd ..");
      expect(term.execute("pwd").output).toBe("/Users/user");
    });
  });

  /* ── File Operations ────────── */
  describe("File System", () => {
    test("touch creates a file", () => {
      term.execute("touch test.txt");
      const r = term.execute("cat test.txt");
      expect(r.exitCode).toBe(0);
      expect(r.output).toBe("");
    });

    test("cat missing file returns error", () => {
      const r = term.execute("cat nope.txt");
      expect(r.exitCode).toBe(1);
      expect(r.output).toContain("No such file");
    });

    test("mkdir creates directory marker", () => {
      const r = term.execute("mkdir mydir");
      expect(r.exitCode).toBe(0);
    });

    test("rm deletes a file", () => {
      term.execute("touch temp.txt");
      expect(term.execute("rm temp.txt").exitCode).toBe(0);
      expect(term.execute("cat temp.txt").exitCode).toBe(1);
    });

    test("rm missing file returns error", () => {
      expect(term.execute("rm ghost.txt").exitCode).toBe(1);
    });

    test("ls lists files in cwd", () => {
      term.execute("touch file1.txt");
      term.execute("touch file2.txt");
      const r = term.execute("ls");
      expect(r.output).toContain("file1.txt");
      expect(r.output).toContain("file2.txt");
    });
  });

  /* ── History ────────────────── */
  describe("History", () => {
    test("commands are recorded in history", () => {
      term.execute("echo a");
      term.execute("echo b");
      expect(term.getHistory()).toEqual(["echo a", "echo b"]);
    });

    test("duplicate consecutive commands not duplicated", () => {
      term.execute("echo a");
      term.execute("echo a");
      expect(term.getHistory()).toEqual(["echo a"]);
    });

    test("historyUp/historyDown navigates", () => {
      term.execute("first");
      term.execute("second");
      term.execute("third");
      expect(term.historyUp()).toBe("third");
      expect(term.historyUp()).toBe("second");
      expect(term.historyDown()).toBe("third");
      expect(term.historyDown()).toBe("");
    });

    test("clearHistory resets", () => {
      term.execute("echo x");
      term.clearHistory();
      expect(term.getHistory()).toHaveLength(0);
    });

    test("history command shows list", () => {
      term.execute("echo a");
      term.execute("echo b");
      const r = term.execute("history");
      expect(r.output).toContain("echo a");
      expect(r.output).toContain("echo b");
    });
  });

  /* ── Prompt ─────────────────── */
  describe("Prompt", () => {
    test("prompt shows ~ for home directory", () => {
      expect(term.getPrompt()).toContain("~");
    });

    test("prompt shows directory name after cd", () => {
      term.execute("cd /etc");
      expect(term.getPrompt()).toContain("etc");
    });
  });

  /* ── Environment Variables ──── */
  describe("Environment", () => {
    test("env shows variables", () => {
      const r = term.execute("env");
      expect(r.output).toContain("HOME=/Users/user");
    });

    test("export sets variable", () => {
      term.execute("export FOO=bar");
      expect(term.env.FOO).toBe("bar");
    });

    test("export without = returns error", () => {
      expect(term.execute("export INVALID").exitCode).toBe(1);
    });
  });

  /* ── Aliases ────────────────── */
  describe("Aliases", () => {
    test("alias sets and uses alias", () => {
      term.execute("alias ll=ls");
      expect(term.aliases.ll).toBe("ls");
    });

    test("alias command without args lists aliases", () => {
      term.execute("alias ll=ls");
      const r = term.execute("alias");
      expect(r.output).toContain("ll");
    });
  });

  /* ── Output ─────────────────── */
  describe("Output Tracking", () => {
    test("getOutput returns all command outputs", () => {
      term.execute("echo hello");
      term.execute("pwd");
      expect(term.getOutput()).toHaveLength(2);
      expect(term.getOutput()[0].input).toBe("echo hello");
    });

    test("clear resets output", () => {
      term.execute("echo a");
      term.execute("clear");
      expect(term.getOutput()).toHaveLength(0);
    });
  });
});
