const { NetworkManager } = require("../modules/network-manager");

describe("NetworkManager", () => {
  let nm;
  beforeEach(() => { nm = new NetworkManager(); });

  /* ── Wi-Fi Toggle ───────────── */
  describe("Wi-Fi", () => {
    test("starts enabled", () => {
      expect(nm.isWifiEnabled()).toBe(true);
    });

    test("disable/enable wifi", () => {
      nm.disableWifi();
      expect(nm.isWifiEnabled()).toBe(false);
      expect(nm.isConnected()).toBe(false);
      nm.enableWifi();
      expect(nm.isWifiEnabled()).toBe(true);
    });

    test("disabling wifi disconnects", () => {
      const n = nm.addNetwork({ ssid: "Home", security: "Open" });
      nm.connect(n.id);
      nm.disableWifi();
      expect(nm.isConnected()).toBe(false);
    });
  });

  /* ── Network Scanning ───────── */
  describe("Networks", () => {
    test("addNetwork creates network", () => {
      const n = nm.addNetwork({ ssid: "MyWiFi", signal: -45, security: "WPA2" });
      expect(n.ssid).toBe("MyWiFi");
      expect(n.signal).toBe(-45);
    });

    test("scanNetworks returns sorted by signal", () => {
      nm.addNetwork({ ssid: "Weak", signal: -80 });
      nm.addNetwork({ ssid: "Strong", signal: -30 });
      nm.addNetwork({ ssid: "Medium", signal: -60 });
      const list = nm.scanNetworks();
      expect(list[0].ssid).toBe("Strong");
      expect(list[2].ssid).toBe("Weak");
    });

    test("getNetworkBySSID finds network", () => {
      nm.addNetwork({ ssid: "Test" });
      expect(nm.getNetworkBySSID("Test")).not.toBeNull();
      expect(nm.getNetworkBySSID("Nope")).toBeNull();
    });
  });

  /* ── Connection ─────────────── */
  describe("Connection", () => {
    test("connect to open network without password", () => {
      const n = nm.addNetwork({ ssid: "Open", security: "Open" });
      const r = nm.connect(n.id);
      expect(r.success).toBe(true);
      expect(nm.isConnected()).toBe(true);
    });

    test("connect to secured network requires password", () => {
      const n = nm.addNetwork({ ssid: "Secure", security: "WPA2" });
      const r = nm.connect(n.id);
      expect(r.success).toBe(false);
      expect(r.error).toContain("Password required");
    });

    test("connect with password succeeds", () => {
      const n = nm.addNetwork({ ssid: "Secure", security: "WPA2" });
      const r = nm.connect(n.id, "secret123");
      expect(r.success).toBe(true);
    });

    test("connect auto-saves password", () => {
      const n = nm.addNetwork({ ssid: "AutoSave", security: "WPA2" });
      nm.connect(n.id, "pass123");
      expect(nm.getSavedNetworks()).toHaveLength(1);
    });

    test("connect fails when wifi disabled", () => {
      const n = nm.addNetwork({ ssid: "X", security: "Open" });
      nm.disableWifi();
      expect(nm.connect(n.id).success).toBe(false);
    });

    test("connect uses saved password", () => {
      const n = nm.addNetwork({ ssid: "Saved", security: "WPA2" });
      nm.saveNetwork("Saved", "mypass");
      const r = nm.connect(n.id);
      expect(r.success).toBe(true);
    });

    test("disconnect clears connection", () => {
      const n = nm.addNetwork({ ssid: "Net", security: "Open" });
      nm.connect(n.id);
      nm.disconnect();
      expect(nm.isConnected()).toBe(false);
      expect(nm.getConnectedNetwork()).toBeNull();
    });

    test("getConnectedNetwork returns current", () => {
      const n = nm.addNetwork({ ssid: "Current", security: "Open" });
      nm.connect(n.id);
      expect(nm.getConnectedNetwork().ssid).toBe("Current");
    });
  });

  /* ── Saved Networks ─────────── */
  describe("Saved Networks", () => {
    test("saveNetwork stores credentials", () => {
      nm.saveNetwork("Home", "pass123");
      expect(nm.getSavedNetworks()).toHaveLength(1);
      expect(nm.getSavedNetworks()[0].autoJoin).toBe(true);
    });

    test("saveNetwork updates existing", () => {
      nm.saveNetwork("Home", "old");
      nm.saveNetwork("Home", "new");
      expect(nm.getSavedNetworks()).toHaveLength(1);
      expect(nm.getSavedNetworks()[0].password).toBe("new");
    });

    test("forgetNetwork removes saved", () => {
      nm.saveNetwork("Home", "pass");
      expect(nm.forgetNetwork("Home")).toBe(true);
      expect(nm.getSavedNetworks()).toHaveLength(0);
    });

    test("forgetNetwork returns false for missing", () => {
      expect(nm.forgetNetwork("Nope")).toBe(false);
    });
  });

  /* ── Signal Quality ─────────── */
  describe("Signal Quality", () => {
    test("excellent signal", () => {
      const n = nm.addNetwork({ ssid: "A", signal: -30 });
      expect(nm.getSignalQuality(n.id).label).toBe("Excellent");
    });

    test("good signal", () => {
      const n = nm.addNetwork({ ssid: "B", signal: -60 });
      expect(nm.getSignalQuality(n.id).label).toBe("Good");
    });

    test("fair signal", () => {
      const n = nm.addNetwork({ ssid: "C", signal: -65 });
      expect(nm.getSignalQuality(n.id).label).toBe("Fair");
    });

    test("weak signal", () => {
      const n = nm.addNetwork({ ssid: "D", signal: -80 });
      expect(nm.getSignalQuality(n.id).label).toBe("Weak");
    });

    test("returns null for missing network", () => {
      expect(nm.getSignalQuality(999)).toBeNull();
    });
  });

  /* ── Diagnostics ────────────── */
  describe("Diagnostics", () => {
    test("runDiagnostics returns status", () => {
      const n = nm.addNetwork({ ssid: "Net", security: "Open" });
      nm.connect(n.id);
      const d = nm.runDiagnostics();
      expect(d.connected).toBe(true);
      expect(d.network).toBe("Net");
      expect(d.issues).toHaveLength(0);
    });

    test("reports issues when not connected", () => {
      const d = nm.runDiagnostics();
      expect(d.issues).toContain("Not connected to any network");
    });

    test("reports issues when wifi disabled", () => {
      nm.disableWifi();
      const d = nm.runDiagnostics();
      expect(d.issues).toContain("Wi-Fi is disabled");
    });

    test("getDiagnosticsLog accumulates", () => {
      nm.runDiagnostics();
      nm.runDiagnostics();
      expect(nm.getDiagnosticsLog()).toHaveLength(2);
    });
  });

  /* ── DNS ────────────────────── */
  describe("DNS", () => {
    test("default DNS servers", () => {
      expect(nm.getDnsServers()).toEqual(["8.8.8.8", "8.8.4.4"]);
    });

    test("setDnsServers updates", () => {
      nm.setDnsServers(["1.1.1.1"]);
      expect(nm.getDnsServers()).toEqual(["1.1.1.1"]);
    });
  });

  /* ── VPN ────────────────────── */
  describe("VPN", () => {
    test("connectVpn creates connection", () => {
      const v = nm.connectVpn({ name: "My VPN", server: "vpn.example.com" });
      expect(v.connected).toBe(true);
      expect(v.name).toBe("My VPN");
    });

    test("disconnectVpn clears", () => {
      nm.connectVpn({ name: "VPN" });
      expect(nm.disconnectVpn()).toBe(true);
      expect(nm.getVpnStatus()).toBeNull();
    });
  });

  /* ── Connection Info ────────── */
  describe("Connection Info", () => {
    test("getConnectionInfo returns full details", () => {
      const n = nm.addNetwork({ ssid: "Full", security: "WPA2", channel: 44 });
      nm.connect(n.id, "pass");
      const info = nm.getConnectionInfo();
      expect(info.ssid).toBe("Full");
      expect(info.channel).toBe(44);
    });

    test("returns null when not connected", () => {
      expect(nm.getConnectionInfo()).toBeNull();
    });
  });
});
