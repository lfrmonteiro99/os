/* ── Bluetooth Manager ───────────────────────────── */
/* macOS-style Bluetooth device management            */

class BluetoothManager {
  constructor(options) {
    options = options || {};
    this.enabled = options.enabled || false;
    this.discoverable = options.discoverable || false;
    this.discovering = false;
    this.devices = new Map();
    this.pairedDevices = new Set();
    this.connectedDevices = new Set();
    this.maxConnections = 7;
    this.listeners = { deviceFound: [], connectionChange: [], stateChange: [] };
    this.history = [];
  }

  /* ── Enable / Disable ──────────── */
  enable() {
    this.enabled = true;
    this._emitStateChange("enabled");
    return true;
  }

  disable() {
    this.enabled = false;
    this.discovering = false;
    var self = this;
    this.connectedDevices.forEach(function (id) {
      self._emitConnectionChange(id, "disconnected");
    });
    this.connectedDevices.clear();
    this._emitStateChange("disabled");
    return true;
  }

  /* ── Discovery ─────────────────── */
  startDiscovery() {
    if (!this.enabled) return { success: false, error: "Bluetooth is disabled" };
    this.discovering = true;
    return { success: true };
  }

  stopDiscovery() {
    this.discovering = false;
    return true;
  }

  discoverDevices() {
    if (!this.discovering) return [];
    var mockDevices = [
      { id: "mock-hp-1", name: "AirPods Pro", type: "headphones", signal: -35 },
      { id: "mock-kb-1", name: "Magic Keyboard", type: "keyboard", signal: -50 },
      { id: "mock-ms-1", name: "Magic Mouse", type: "mouse", signal: -45 },
      { id: "mock-sp-1", name: "HomePod Mini", type: "speaker", signal: -55 },
      { id: "mock-ph-1", name: "iPhone 15", type: "phone", signal: -40 },
      { id: "mock-wt-1", name: "Apple Watch", type: "watch", signal: -60 },
    ];
    var self = this;
    mockDevices.forEach(function (d) {
      if (!self.devices.has(d.id)) {
        self.addDevice(d);
      }
      self.listeners.deviceFound.forEach(function (cb) { cb(d); });
    });
    return mockDevices;
  }

  /* ── Device Management ─────────── */
  addDevice(opts) {
    var device = {
      id: opts.id,
      name: opts.name || "Unknown Device",
      type: opts.type || "unknown",
      signal: opts.signal || -50,
      battery: opts.battery !== undefined ? opts.battery : null,
      autoConnect: false,
    };
    this.devices.set(device.id, device);
    return device;
  }

  getDevice(id) {
    return this.devices.get(id) || null;
  }

  /* ── Pairing ───────────────────── */
  pair(deviceId) {
    if (!this.enabled) return { success: false, error: "Bluetooth is disabled" };
    var device = this.devices.get(deviceId);
    if (!device) return { success: false, error: "Device not found" };
    this.pairedDevices.add(deviceId);
    return { status: "paired", deviceId: deviceId };
  }

  unpair(deviceId) {
    if (!this.pairedDevices.has(deviceId)) return false;
    if (this.connectedDevices.has(deviceId)) {
      this.disconnect(deviceId);
    }
    this.pairedDevices.delete(deviceId);
    return true;
  }

  /* ── Connect / Disconnect ──────── */
  connect(deviceId) {
    if (!this.enabled) return { success: false, error: "Bluetooth is disabled" };
    if (!this.pairedDevices.has(deviceId)) return { success: false, error: "Device is not paired" };
    if (this.connectedDevices.size >= this.maxConnections) {
      return { success: false, error: "Cannot connect: max connections reached" };
    }
    this.connectedDevices.add(deviceId);
    this.history.push({ deviceId: deviceId, timestamp: Date.now(), action: "connected" });
    this._emitConnectionChange(deviceId, "connected");
    return { success: true, deviceId: deviceId };
  }

  disconnect(deviceId) {
    if (!this.connectedDevices.has(deviceId)) return false;
    this.connectedDevices.delete(deviceId);
    this._emitConnectionChange(deviceId, "disconnected");
    return true;
  }

  /* ── Device Queries ────────────── */
  getConnectedDevices() {
    var self = this;
    var result = [];
    this.connectedDevices.forEach(function (id) {
      var dev = self.devices.get(id);
      if (dev) result.push(dev);
    });
    return result;
  }

  getPairedDevices() {
    var self = this;
    var result = [];
    this.pairedDevices.forEach(function (id) {
      var dev = self.devices.get(id);
      if (dev) result.push(dev);
    });
    return result;
  }

  /* ── Battery Level ─────────────── */
  getBatteryLevel(deviceId) {
    if (!this.connectedDevices.has(deviceId)) return null;
    var device = this.devices.get(deviceId);
    if (!device) return null;
    return device.battery;
  }

  /* ── Rename Device ─────────────── */
  renameDevice(deviceId, newName) {
    var device = this.devices.get(deviceId);
    if (!device) return false;
    device.name = newName;
    return true;
  }

  /* ── Auto-Connect ──────────────── */
  setAutoConnect(deviceId, value) {
    var device = this.devices.get(deviceId);
    if (!device) return false;
    device.autoConnect = value;
    return true;
  }

  /* ── Signal Strength ───────────── */
  getSignalCategory(deviceId) {
    var device = this.devices.get(deviceId);
    if (!device) return null;
    var signal = device.signal;
    if (signal >= -45) return "strong";
    if (signal >= -65) return "medium";
    return "weak";
  }

  /* ── History ───────────────────── */
  getHistory() {
    return this.history.slice();
  }

  /* ── Callbacks ─────────────────── */
  onDeviceFound(callback) {
    this.listeners.deviceFound.push(callback);
  }

  onConnectionChange(callback) {
    this.listeners.connectionChange.push(callback);
  }

  onStateChange(callback) {
    this.listeners.stateChange.push(callback);
  }

  /* ── Internal Emitters ─────────── */
  _emitConnectionChange(deviceId, type) {
    var event = { deviceId: deviceId, type: type, timestamp: Date.now() };
    this.listeners.connectionChange.forEach(function (cb) { cb(event); });
  }

  _emitStateChange(state) {
    var event = { state: state, timestamp: Date.now() };
    this.listeners.stateChange.forEach(function (cb) { cb(event); });
  }
}

if (typeof module !== "undefined") module.exports = { BluetoothManager: BluetoothManager };
