/* ── Network Manager ──────────────────────────────── */
/* macOS-style Wi-Fi, network diagnostics, and profiles */

class NetworkManager {
  constructor() {
    this.networks = [];
    this.nextId = 1;
    this.connectedId = null;
    this.wifiEnabled = true;
    this.savedNetworks = [];    // { ssid, password, autoJoin }
    this.diagnosticsLog = [];
    this.dnsServers = ["8.8.8.8", "8.8.4.4"];
    this.proxy = null;
    this.vpn = null;
  }

  /* ── Wi-Fi Toggle ─────────────── */
  enableWifi() {
    this.wifiEnabled = true;
    return true;
  }

  disableWifi() {
    this.wifiEnabled = false;
    this.connectedId = null;
    return true;
  }

  isWifiEnabled() {
    return this.wifiEnabled;
  }

  /* ── Network Scanning ─────────── */
  addNetwork(opts) {
    var net = {
      id: this.nextId++,
      ssid: opts.ssid || "Unknown",
      bssid: opts.bssid || "00:00:00:00:00:00",
      signal: opts.signal || -50,       // dBm: -30 excellent, -67 good, -70 fair, -80 weak
      security: opts.security || "WPA2",  // Open, WEP, WPA, WPA2, WPA3
      frequency: opts.frequency || 5,     // GHz: 2.4 or 5
      channel: opts.channel || 36,
      hidden: opts.hidden || false,
    };
    this.networks.push(net);
    return net;
  }

  scanNetworks() {
    return this.networks.slice().sort(function (a, b) { return b.signal - a.signal; });
  }

  getNetwork(id) {
    return this.networks.find(function (n) { return n.id === id; }) || null;
  }

  getNetworkBySSID(ssid) {
    return this.networks.find(function (n) { return n.ssid === ssid; }) || null;
  }

  /* ── Connection ───────────────── */
  connect(id, password) {
    if (!this.wifiEnabled) return { success: false, error: "Wi-Fi is disabled" };
    var net = this.getNetwork(id);
    if (!net) return { success: false, error: "Network not found" };
    if (net.security !== "Open" && !password) {
      // Check saved networks
      var saved = this.savedNetworks.find(function (s) { return s.ssid === net.ssid; });
      if (!saved) return { success: false, error: "Password required" };
      password = saved.password;
    }
    this.connectedId = id;
    // Auto-save
    if (password) this.saveNetwork(net.ssid, password);
    return { success: true, network: net };
  }

  disconnect() {
    this.connectedId = null;
    return true;
  }

  getConnectedNetwork() {
    if (!this.connectedId) return null;
    return this.getNetwork(this.connectedId);
  }

  isConnected() {
    return this.connectedId !== null;
  }

  /* ── Saved Networks ───────────── */
  saveNetwork(ssid, password, autoJoin) {
    var existing = this.savedNetworks.find(function (s) { return s.ssid === ssid; });
    if (existing) {
      existing.password = password;
      if (autoJoin !== undefined) existing.autoJoin = autoJoin;
      return existing;
    }
    var entry = { ssid: ssid, password: password, autoJoin: autoJoin !== false };
    this.savedNetworks.push(entry);
    return entry;
  }

  forgetNetwork(ssid) {
    var idx = this.savedNetworks.findIndex(function (s) { return s.ssid === ssid; });
    if (idx === -1) return false;
    this.savedNetworks.splice(idx, 1);
    return true;
  }

  getSavedNetworks() {
    return this.savedNetworks.slice();
  }

  /* ── Signal Quality ───────────── */
  getSignalQuality(id) {
    var net = this.getNetwork(id);
    if (!net) return null;
    var signal = net.signal;
    if (signal >= -30) return { bars: 4, label: "Excellent" };
    if (signal >= -50) return { bars: 4, label: "Excellent" };
    if (signal >= -60) return { bars: 3, label: "Good" };
    if (signal >= -70) return { bars: 2, label: "Fair" };
    return { bars: 1, label: "Weak" };
  }

  /* ── Network Diagnostics ──────── */
  runDiagnostics() {
    var connected = this.getConnectedNetwork();
    var results = {
      timestamp: Date.now(),
      wifiEnabled: this.wifiEnabled,
      connected: !!connected,
      network: connected ? connected.ssid : null,
      signal: connected ? this.getSignalQuality(connected.id) : null,
      dns: this.dnsServers.slice(),
      proxy: this.proxy,
      vpn: this.vpn,
      issues: [],
    };

    if (!this.wifiEnabled) results.issues.push("Wi-Fi is disabled");
    if (!connected) results.issues.push("Not connected to any network");
    if (connected && connected.signal < -75) results.issues.push("Weak signal strength");

    this.diagnosticsLog.push(results);
    return results;
  }

  getDiagnosticsLog() {
    return this.diagnosticsLog.slice();
  }

  /* ── DNS ──────────────────────── */
  setDnsServers(servers) {
    this.dnsServers = servers.slice();
    return this.dnsServers;
  }

  getDnsServers() {
    return this.dnsServers.slice();
  }

  /* ── VPN ──────────────────────── */
  connectVpn(config) {
    this.vpn = {
      name: config.name || "VPN",
      server: config.server || "",
      protocol: config.protocol || "IKEv2",
      connected: true,
      connectedAt: Date.now(),
    };
    return this.vpn;
  }

  disconnectVpn() {
    if (!this.vpn) return false;
    this.vpn.connected = false;
    this.vpn = null;
    return true;
  }

  getVpnStatus() {
    return this.vpn ? Object.assign({}, this.vpn) : null;
  }

  /* ── Proxy ────────────────────── */
  setProxy(config) {
    this.proxy = {
      type: config.type || "HTTP",
      host: config.host || "",
      port: config.port || 8080,
      enabled: true,
    };
    return this.proxy;
  }

  removeProxy() {
    this.proxy = null;
    return true;
  }

  /* ── Connection Info ──────────── */
  getConnectionInfo() {
    var connected = this.getConnectedNetwork();
    if (!connected) return null;
    return {
      ssid: connected.ssid,
      bssid: connected.bssid,
      signal: connected.signal,
      security: connected.security,
      frequency: connected.frequency,
      channel: connected.channel,
      dns: this.dnsServers,
      vpn: this.vpn ? this.vpn.name : null,
      proxy: this.proxy ? this.proxy.host + ":" + this.proxy.port : null,
    };
  }
}

if (typeof module !== "undefined") module.exports = { NetworkManager: NetworkManager };
