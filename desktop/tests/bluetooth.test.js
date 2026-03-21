const { BluetoothManager } = require("../modules/bluetooth");

describe("BluetoothManager", () => {
  let bt;
  beforeEach(() => { bt = new BluetoothManager(); });

  /* ── Constructor Defaults ────── */
  describe("Constructor", () => {
    test("starts disabled", () => {
      expect(bt.enabled).toBe(false);
    });

    test("starts not discoverable", () => {
      expect(bt.discoverable).toBe(false);
    });

    test("starts with no devices", () => {
      expect(bt.devices.size).toBe(0);
    });

    test("accepts options", () => {
      const bt2 = new BluetoothManager({ enabled: true, discoverable: true });
      expect(bt2.enabled).toBe(true);
      expect(bt2.discoverable).toBe(true);
    });
  });

  /* ── Enable / Disable ────────── */
  describe("Enable/Disable", () => {
    test("enable turns bluetooth on", () => {
      bt.enable();
      expect(bt.enabled).toBe(true);
    });

    test("disable turns bluetooth off", () => {
      bt.enable();
      bt.disable();
      expect(bt.enabled).toBe(false);
    });

    test("disable stops discovery", () => {
      bt.enable();
      bt.startDiscovery();
      bt.disable();
      expect(bt.discovering).toBe(false);
    });

    test("disable disconnects all devices", () => {
      bt.enable();
      const d = bt.addDevice({ id: "dev1", name: "Speaker", type: "speaker", signal: -40 });
      bt.pair("dev1");
      bt.connect("dev1");
      bt.disable();
      expect(bt.getConnectedDevices()).toHaveLength(0);
    });
  });

  /* ── Discovery ───────────────── */
  describe("Discovery", () => {
    test("startDiscovery sets discovering true", () => {
      bt.enable();
      bt.startDiscovery();
      expect(bt.discovering).toBe(true);
    });

    test("startDiscovery fails when disabled", () => {
      const r = bt.startDiscovery();
      expect(r.success).toBe(false);
      expect(r.error).toContain("disabled");
    });

    test("stopDiscovery sets discovering false", () => {
      bt.enable();
      bt.startDiscovery();
      bt.stopDiscovery();
      expect(bt.discovering).toBe(false);
    });

    test("discoverDevices returns mock devices", () => {
      bt.enable();
      bt.startDiscovery();
      const found = bt.discoverDevices();
      expect(found.length).toBeGreaterThan(0);
      expect(found[0]).toHaveProperty("name");
      expect(found[0]).toHaveProperty("type");
      expect(found[0]).toHaveProperty("signal");
    });
  });

  /* ── Device Types ────────────── */
  describe("Device Types", () => {
    test("supports headphones, keyboard, mouse, speaker, phone, watch", () => {
      bt.enable();
      const types = ["headphones", "keyboard", "mouse", "speaker", "phone", "watch"];
      types.forEach((type, i) => {
        bt.addDevice({ id: "t" + i, name: "Dev " + i, type: type, signal: -40 });
      });
      expect(bt.devices.size).toBe(6);
      const dev = bt.getDevice("t0");
      expect(dev.type).toBe("headphones");
    });
  });

  /* ── Pairing ─────────────────── */
  describe("Pairing", () => {
    beforeEach(() => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "AirPods", type: "headphones", signal: -35 });
    });

    test("pair returns pairing flow result", () => {
      const r = bt.pair("dev1");
      expect(r.status).toBe("paired");
      expect(r.deviceId).toBe("dev1");
    });

    test("pair adds device to pairedDevices", () => {
      bt.pair("dev1");
      expect(bt.pairedDevices.has("dev1")).toBe(true);
    });

    test("pair fails for unknown device", () => {
      const r = bt.pair("unknown");
      expect(r.success).toBe(false);
      expect(r.error).toContain("not found");
    });

    test("pair fails when bluetooth disabled", () => {
      bt.disable();
      const r = bt.pair("dev1");
      expect(r.success).toBe(false);
    });

    test("unpair removes device from paired set", () => {
      bt.pair("dev1");
      const r = bt.unpair("dev1");
      expect(r).toBe(true);
      expect(bt.pairedDevices.has("dev1")).toBe(false);
    });

    test("unpair disconnects device first", () => {
      bt.pair("dev1");
      bt.connect("dev1");
      bt.unpair("dev1");
      expect(bt.connectedDevices.has("dev1")).toBe(false);
    });

    test("unpair returns false for non-paired device", () => {
      expect(bt.unpair("dev1")).toBe(false);
    });
  });

  /* ── Connect / Disconnect ────── */
  describe("Connect/Disconnect", () => {
    beforeEach(() => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "Keyboard", type: "keyboard", signal: -50 });
      bt.pair("dev1");
    });

    test("connect succeeds for paired device", () => {
      const r = bt.connect("dev1");
      expect(r.success).toBe(true);
      expect(bt.connectedDevices.has("dev1")).toBe(true);
    });

    test("connect fails for unpaired device", () => {
      bt.addDevice({ id: "dev2", name: "Mouse", type: "mouse", signal: -45 });
      const r = bt.connect("dev2");
      expect(r.success).toBe(false);
      expect(r.error).toContain("not paired");
    });

    test("connect fails when bluetooth disabled", () => {
      bt.disable();
      const r = bt.connect("dev1");
      expect(r.success).toBe(false);
    });

    test("disconnect removes from connected set", () => {
      bt.connect("dev1");
      const r = bt.disconnect("dev1");
      expect(r).toBe(true);
      expect(bt.connectedDevices.has("dev1")).toBe(false);
    });

    test("disconnect returns false for non-connected device", () => {
      expect(bt.disconnect("dev1")).toBe(false);
    });

    test("connect adds to history", () => {
      bt.connect("dev1");
      expect(bt.history.length).toBe(1);
      expect(bt.history[0].deviceId).toBe("dev1");
    });
  });

  /* ── Device Queries ──────────── */
  describe("Device Queries", () => {
    beforeEach(() => {
      bt.enable();
      bt.addDevice({ id: "d1", name: "Headphones", type: "headphones", signal: -30 });
      bt.addDevice({ id: "d2", name: "Keyboard", type: "keyboard", signal: -55 });
      bt.addDevice({ id: "d3", name: "Mouse", type: "mouse", signal: -45 });
      bt.pair("d1");
      bt.pair("d2");
      bt.connect("d1");
    });

    test("getConnectedDevices returns connected only", () => {
      const connected = bt.getConnectedDevices();
      expect(connected).toHaveLength(1);
      expect(connected[0].id).toBe("d1");
    });

    test("getPairedDevices returns all paired", () => {
      const paired = bt.getPairedDevices();
      expect(paired).toHaveLength(2);
    });
  });

  /* ── Battery Level ───────────── */
  describe("Battery Level", () => {
    test("returns battery for connected device", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "AirPods", type: "headphones", signal: -35, battery: 85 });
      bt.pair("dev1");
      bt.connect("dev1");
      const level = bt.getBatteryLevel("dev1");
      expect(level).toBe(85);
    });

    test("returns null for non-connected device", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "AirPods", type: "headphones", signal: -35, battery: 85 });
      expect(bt.getBatteryLevel("dev1")).toBeNull();
    });
  });

  /* ── Rename Device ───────────── */
  describe("Rename Device", () => {
    test("renameDevice changes device name", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "Old Name", type: "speaker", signal: -40 });
      const r = bt.renameDevice("dev1", "New Name");
      expect(r).toBe(true);
      expect(bt.getDevice("dev1").name).toBe("New Name");
    });

    test("renameDevice fails for unknown device", () => {
      expect(bt.renameDevice("nope", "Name")).toBe(false);
    });
  });

  /* ── Auto-Connect ────────────── */
  describe("Auto-Connect", () => {
    test("setAutoConnect stores preference", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "Watch", type: "watch", signal: -50 });
      bt.pair("dev1");
      bt.setAutoConnect("dev1", true);
      expect(bt.getDevice("dev1").autoConnect).toBe(true);
    });

    test("setAutoConnect defaults to false", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "Phone", type: "phone", signal: -40 });
      expect(bt.getDevice("dev1").autoConnect).toBe(false);
    });

    test("setAutoConnect fails for unknown device", () => {
      expect(bt.setAutoConnect("nope", true)).toBe(false);
    });
  });

  /* ── Signal Strength ─────────── */
  describe("Signal Strength", () => {
    test("strong signal category", () => {
      bt.enable();
      bt.addDevice({ id: "d1", name: "Near", type: "speaker", signal: -30 });
      expect(bt.getSignalCategory("d1")).toBe("strong");
    });

    test("medium signal category", () => {
      bt.enable();
      bt.addDevice({ id: "d2", name: "Mid", type: "speaker", signal: -55 });
      expect(bt.getSignalCategory("d2")).toBe("medium");
    });

    test("weak signal category", () => {
      bt.enable();
      bt.addDevice({ id: "d3", name: "Far", type: "speaker", signal: -80 });
      expect(bt.getSignalCategory("d3")).toBe("weak");
    });

    test("returns null for unknown device", () => {
      expect(bt.getSignalCategory("nope")).toBeNull();
    });
  });

  /* ── Device History ──────────── */
  describe("History", () => {
    test("getHistory returns recently connected devices", () => {
      bt.enable();
      bt.addDevice({ id: "d1", name: "A", type: "headphones", signal: -40 });
      bt.addDevice({ id: "d2", name: "B", type: "keyboard", signal: -50 });
      bt.pair("d1");
      bt.pair("d2");
      bt.connect("d1");
      bt.connect("d2");
      const h = bt.getHistory();
      expect(h).toHaveLength(2);
      expect(h[0]).toHaveProperty("deviceId");
      expect(h[0]).toHaveProperty("timestamp");
    });
  });

  /* ── Max Connections ─────────── */
  describe("Max Connections", () => {
    test("enforces max 7 connections", () => {
      bt.enable();
      for (let i = 0; i < 8; i++) {
        bt.addDevice({ id: "d" + i, name: "Dev " + i, type: "speaker", signal: -40 });
        bt.pair("d" + i);
      }
      for (let i = 0; i < 7; i++) {
        const r = bt.connect("d" + i);
        expect(r.success).toBe(true);
      }
      const r = bt.connect("d7");
      expect(r.success).toBe(false);
      expect(r.error).toContain("max");
    });

    test("maxConnections defaults to 7", () => {
      expect(bt.maxConnections).toBe(7);
    });
  });

  /* ── Callbacks ───────────────── */
  describe("Callbacks", () => {
    test("onDeviceFound fires during discovery", () => {
      bt.enable();
      const found = [];
      bt.onDeviceFound((device) => { found.push(device); });
      bt.startDiscovery();
      bt.discoverDevices();
      expect(found.length).toBeGreaterThan(0);
    });

    test("onConnectionChange fires on connect", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "KB", type: "keyboard", signal: -40 });
      bt.pair("dev1");
      const changes = [];
      bt.onConnectionChange((ev) => { changes.push(ev); });
      bt.connect("dev1");
      expect(changes).toHaveLength(1);
      expect(changes[0].type).toBe("connected");
      expect(changes[0].deviceId).toBe("dev1");
    });

    test("onConnectionChange fires on disconnect", () => {
      bt.enable();
      bt.addDevice({ id: "dev1", name: "KB", type: "keyboard", signal: -40 });
      bt.pair("dev1");
      bt.connect("dev1");
      const changes = [];
      bt.onConnectionChange((ev) => { changes.push(ev); });
      bt.disconnect("dev1");
      expect(changes).toHaveLength(1);
      expect(changes[0].type).toBe("disconnected");
    });
  });
});
