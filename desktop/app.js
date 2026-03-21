/* ==========================================================
   AuroraOS Desktop — app.js
   macOS Big Sur interactions & window management
   ========================================================== */

(function () {
  "use strict";

  /* ── 1. Clock ──────────────────────────────────────── */
  const clockEl = document.getElementById("clock");
  const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  const MONTHS = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
  ];

  function updateClock() {
    const now = new Date();
    const day = DAYS[now.getDay()];
    const mon = MONTHS[now.getMonth()];
    const date = now.getDate();
    let hours = now.getHours();
    const ampm = hours >= 12 ? "PM" : "AM";
    hours = hours % 12 || 12;
    const mins = String(now.getMinutes()).padStart(2, "0");
    clockEl.textContent = day + " " + mon + " " + date + "  " + hours + ":" + mins + " " + ampm;
  }

  updateClock();
  setInterval(updateClock, 1000);

  // Update calendar day
  const calDay = document.getElementById("calendar-day");
  if (calDay) {
    calDay.textContent = new Date().getDate();
  }

  /* ── 2. Control Center Toggle ──────────────────────── */
  const ccPanel = document.getElementById("control-center");
  const ccToggle = document.getElementById("cc-toggle");

  ccToggle.addEventListener("click", function (e) {
    e.stopPropagation();
    ccPanel.classList.toggle("visible");
    // Close spotlight if open
    spotlightOverlay.classList.remove("visible");
  });

  document.addEventListener("click", function (e) {
    if (!ccPanel.contains(e.target) && e.target !== ccToggle) {
      ccPanel.classList.remove("visible");
    }
  });

  // CC tile toggles
  document.querySelectorAll("[data-cc-toggle]").forEach(function (tile) {
    tile.addEventListener("click", function (e) {
      e.stopPropagation();
      // Toggle active state on the icon circle inside
      var iconCircle = tile.querySelector(".cc-tile-icon-circle");
      if (iconCircle) {
        iconCircle.classList.toggle("cc-icon-active");
      }
    });
  });

  // CC sliders — click to adjust
  document.querySelectorAll(".cc-slider-track").forEach(function (track) {
    track.addEventListener("click", function (e) {
      var rect = track.getBoundingClientRect();
      var pct = Math.max(0, Math.min(100, ((e.clientX - rect.left) / rect.width) * 100));
      var fill = track.querySelector(".cc-slider-fill");
      if (fill) fill.style.width = pct + "%";
    });
  });

  /* ── 3. Spotlight ──────────────────────────────────── */
  const spotlightOverlay = document.getElementById("spotlight-overlay");
  const spotlightToggle = document.getElementById("spotlight-toggle");
  const spotlightInput = spotlightOverlay.querySelector(".spotlight-input");

  function toggleSpotlight() {
    spotlightOverlay.classList.toggle("visible");
    ccPanel.classList.remove("visible");
    if (spotlightOverlay.classList.contains("visible")) {
      setTimeout(function () {
        spotlightInput.value = "";
        spotlightInput.focus();
      }, 50);
    }
  }

  spotlightToggle.addEventListener("click", function (e) {
    e.stopPropagation();
    toggleSpotlight();
  });

  spotlightOverlay.addEventListener("click", function (e) {
    if (e.target === spotlightOverlay) {
      spotlightOverlay.classList.remove("visible");
    }
  });

  // Cmd+Space or Ctrl+Space to toggle spotlight
  document.addEventListener("keydown", function (e) {
    if (e.key === " " && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      toggleSpotlight();
    }
    if (e.key === "Escape") {
      spotlightOverlay.classList.remove("visible");
      ccPanel.classList.remove("visible");
    }
  });

  /* ── 4. Window Management ──────────────────────────── */
  let topZ = 10;
  const windows = document.querySelectorAll("[data-window]");

  function focusWindow(win) {
    windows.forEach(function (w) { w.classList.remove("focused"); });
    topZ++;
    win.style.zIndex = topZ;
    win.classList.add("focused");

    // Update menu bar app name
    var title = win.querySelector(".window-title");
    var appName = document.querySelector(".menu-item.bold");
    if (title && appName) {
      appName.textContent = title.textContent;
    }
  }

  windows.forEach(function (win) {
    win.addEventListener("mousedown", function () {
      focusWindow(win);
    });
  });

  // Focus the topmost window initially
  if (windows.length) focusWindow(windows[windows.length - 1]);

  // Drag by title bar
  windows.forEach(function (win) {
    var titlebar = win.querySelector("[data-titlebar]");
    if (!titlebar) return;

    var dragging = false;
    var offsetX = 0;
    var offsetY = 0;

    titlebar.addEventListener("mousedown", function (e) {
      if (e.target.classList.contains("tl-btn")) return;
      if (e.target.tagName === "INPUT") return;

      dragging = true;
      // Handle windows with right positioning
      if (win.style.right && win.style.right !== "auto") {
        win.style.left = win.offsetLeft + "px";
        win.style.right = "auto";
      }
      offsetX = e.clientX - win.offsetLeft;
      offsetY = e.clientY - win.offsetTop;
      focusWindow(win);
      e.preventDefault();
    });

    document.addEventListener("mousemove", function (e) {
      if (!dragging) return;
      var x = Math.max(0, e.clientX - offsetX);
      var y = Math.max(0, e.clientY - offsetY);
      win.style.left = x + "px";
      win.style.top = y + "px";
      win.style.right = "auto";
    });

    document.addEventListener("mouseup", function () {
      dragging = false;
    });
  });

  // Traffic light close button
  document.querySelectorAll(".tl-close").forEach(function (btn) {
    btn.addEventListener("click", function (e) {
      e.stopPropagation();
      var win = btn.closest(".window");
      if (win) {
        win.style.transition = "opacity 0.15s ease, transform 0.15s ease";
        win.style.opacity = "0";
        win.style.transform = "scale(0.95)";
        setTimeout(function () { win.style.display = "none"; }, 160);
      }
    });
  });

  // Traffic light minimize button
  document.querySelectorAll(".tl-minimize").forEach(function (btn) {
    btn.addEventListener("click", function (e) {
      e.stopPropagation();
      var win = btn.closest(".window");
      if (win) {
        win.style.transition = "opacity 0.25s ease, transform 0.25s ease";
        win.style.opacity = "0";
        win.style.transform = "scale(0.3) translateY(200px)";
        setTimeout(function () { win.style.display = "none"; }, 260);
      }
    });
  });

  /* ── 5. Dock Magnification ─────────────────────────── */
  const dock = document.getElementById("dock");
  const dockIcons = dock.querySelectorAll(".dock-icon");

  const EFFECT_DISTANCE = 100;
  const MAX_SCALE = 1.45;

  function resetDockIcons() {
    dockIcons.forEach(function (icon) {
      icon.style.transform = "scale(1)";
      icon.style.marginBottom = "0";
    });
  }

  dock.addEventListener("mousemove", function (e) {
    var mouseX = e.clientX;

    dockIcons.forEach(function (icon) {
      var rect = icon.getBoundingClientRect();
      var iconCenterX = rect.left + rect.width / 2;
      var distance = Math.abs(mouseX - iconCenterX);

      if (distance > EFFECT_DISTANCE) {
        icon.style.transform = "scale(1)";
        icon.style.marginBottom = "0";
        return;
      }

      var ratio = 1 - distance / EFFECT_DISTANCE;
      // Smooth cosine curve for natural magnification
      var t = (1 - Math.cos(ratio * Math.PI)) / 2;
      var scale = 1 + (MAX_SCALE - 1) * t;
      icon.style.transform = "scale(" + scale.toFixed(3) + ")";
      // Lift scaled icons to maintain dock bottom alignment
      icon.style.marginBottom = ((scale - 1) * 25).toFixed(1) + "px";
    });
  });

  dock.addEventListener("mouseleave", resetDockIcons);

  // Dock icon click — bounce animation
  dockIcons.forEach(function (icon) {
    icon.addEventListener("click", function () {
      icon.style.animation = "none";
      // Force reflow
      void icon.offsetHeight;
      icon.style.animation = "dockBounce 0.5s ease";
      setTimeout(function () { icon.style.animation = "none"; }, 500);
    });
  });

  // Add bounce keyframes dynamically
  var bounceStyle = document.createElement("style");
  bounceStyle.textContent =
    "@keyframes dockBounce {" +
    "  0% { transform: translateY(0); }" +
    "  20% { transform: translateY(-16px); }" +
    "  40% { transform: translateY(0); }" +
    "  55% { transform: translateY(-8px); }" +
    "  70% { transform: translateY(0); }" +
    "  82% { transform: translateY(-3px); }" +
    "  100% { transform: translateY(0); }" +
    "}";
  document.head.appendChild(bounceStyle);

  /* ── 6. Menu Bar Interactions ────────────────────────── */
  var menuItems = document.querySelectorAll(".menu-left .menu-item:not(.apple-logo):not(.bold)");
  var activeMenuItem = null;
  var menuDropdown = null;

  var menuData = {
    "File": ["New Window", "New Tab", "Open\u2026", "Close Window", "Close Tab"],
    "Edit": ["Undo", "Redo", "Cut", "Copy", "Paste", "Select All"],
    "View": ["Show Toolbar", "Show Sidebar", "Enter Full Screen"],
    "Go": ["Back", "Forward", "Enclosing Folder", "Home"],
    "Window": ["Minimize", "Zoom", "Bring All to Front"],
    "Help": ["AuroraOS Help", "About AuroraOS"],
  };

  function closeMenuDropdown() {
    if (menuDropdown) {
      menuDropdown.remove();
      menuDropdown = null;
    }
    if (activeMenuItem) {
      activeMenuItem.style.background = "";
      activeMenuItem = null;
    }
  }

  function openMenuDropdown(item) {
    closeMenuDropdown();
    var label = item.textContent;
    var items = menuData[label];
    if (!items) return;

    activeMenuItem = item;
    item.style.background = "rgba(255,255,255,0.2)";

    menuDropdown = document.createElement("div");
    menuDropdown.className = "menu-dropdown";
    var rect = item.getBoundingClientRect();
    menuDropdown.style.left = rect.left + "px";
    menuDropdown.style.top = (rect.bottom + 2) + "px";

    items.forEach(function (text) {
      if (text === "---") {
        var sep = document.createElement("div");
        sep.className = "menu-dropdown-sep";
        menuDropdown.appendChild(sep);
      } else {
        var row = document.createElement("div");
        row.className = "menu-dropdown-item";
        row.textContent = text;
        menuDropdown.appendChild(row);
      }
    });

    document.body.appendChild(menuDropdown);
  }

  menuItems.forEach(function (item) {
    item.addEventListener("click", function (e) {
      e.stopPropagation();
      if (activeMenuItem === item) {
        closeMenuDropdown();
      } else {
        openMenuDropdown(item);
      }
    });

    item.addEventListener("mouseenter", function () {
      if (activeMenuItem && activeMenuItem !== item) {
        openMenuDropdown(item);
      }
    });
  });

  document.addEventListener("click", closeMenuDropdown);

  // Menu dropdown styles
  var menuStyle = document.createElement("style");
  menuStyle.textContent =
    ".menu-dropdown {" +
    "  position: fixed; z-index: 110;" +
    "  min-width: 200px;" +
    "  background: rgba(38,38,38,0.88);" +
    "  backdrop-filter: blur(40px) saturate(180%);" +
    "  -webkit-backdrop-filter: blur(40px) saturate(180%);" +
    "  border-radius: 6px;" +
    "  padding: 4px 0;" +
    "  box-shadow: 0 8px 30px rgba(0,0,0,0.35), 0 0 0 0.5px rgba(255,255,255,0.08);" +
    "  color: #fff; font-size: 13px;" +
    "  animation: menuFadeIn 0.08s ease-out;" +
    "}" +
    "@keyframes menuFadeIn { from { opacity: 0; } to { opacity: 1; } }" +
    ".menu-dropdown-item {" +
    "  padding: 4px 16px; cursor: default;" +
    "  border-radius: 4px; margin: 0 4px;" +
    "}" +
    ".menu-dropdown-item:hover { background: #007aff; }" +
    ".menu-dropdown-sep {" +
    "  height: 0.5px; background: rgba(255,255,255,0.12);" +
    "  margin: 4px 12px;" +
    "}";
  document.head.appendChild(menuStyle);

  /* ── 7. Right-click Context Menu ─────────────────────── */
  document.querySelector(".desktop").addEventListener("contextmenu", function (e) {
    e.preventDefault();
    // Remove any existing context menu
    var existing = document.querySelector(".context-menu");
    if (existing) existing.remove();

    var menu = document.createElement("div");
    menu.className = "context-menu";
    menu.style.left = e.clientX + "px";
    menu.style.top = e.clientY + "px";

    var contextItems = [
      "New Folder", "Get Info", "---",
      "Change Desktop Background\u2026",
      "Use Stacks", "Sort By", "---",
      "Show View Options",
    ];

    contextItems.forEach(function (text) {
      if (text === "---") {
        var sep = document.createElement("div");
        sep.className = "menu-dropdown-sep";
        menu.appendChild(sep);
      } else {
        var row = document.createElement("div");
        row.className = "menu-dropdown-item";
        row.textContent = text;
        menu.appendChild(row);
      }
    });

    document.body.appendChild(menu);

    function removeCtx() {
      menu.remove();
      document.removeEventListener("click", removeCtx);
    }
    setTimeout(function () {
      document.addEventListener("click", removeCtx);
    }, 0);
  });

  // Context menu uses same styles as menu dropdown
  var ctxStyle = document.createElement("style");
  ctxStyle.textContent =
    ".context-menu {" +
    "  position: fixed; z-index: 250;" +
    "  min-width: 220px;" +
    "  background: rgba(38,38,38,0.88);" +
    "  backdrop-filter: blur(40px) saturate(180%);" +
    "  -webkit-backdrop-filter: blur(40px) saturate(180%);" +
    "  border-radius: 6px;" +
    "  padding: 4px 0;" +
    "  box-shadow: 0 8px 30px rgba(0,0,0,0.35), 0 0 0 0.5px rgba(255,255,255,0.08);" +
    "  color: #fff; font-size: 13px;" +
    "  animation: menuFadeIn 0.08s ease-out;" +
    "}";
  document.head.appendChild(ctxStyle);

  /* ══════════════════════════════════════════════════════
     8. WIDGETS PANEL / TODAY VIEW (Issue #14)
     ══════════════════════════════════════════════════════ */
  var widgetsPanel = document.getElementById("widgets-panel");

  // Toggle widgets on clock click
  clockEl.addEventListener("click", function (e) {
    e.stopPropagation();
    widgetsPanel.classList.toggle("visible");
    ccPanel.classList.remove("visible");
    spotlightOverlay.classList.remove("visible");
  });

  // Close widgets on outside click
  document.addEventListener("click", function (e) {
    if (widgetsPanel.classList.contains("visible") &&
        !widgetsPanel.contains(e.target) && e.target !== clockEl) {
      widgetsPanel.classList.remove("visible");
    }
  });

  // Widget clock hands
  function updateWidgetClock() {
    var now = new Date();
    var h = now.getHours() % 12;
    var m = now.getMinutes();
    var s = now.getSeconds();
    var hourDeg = (h * 30) + (m * 0.5);
    var minDeg = m * 6;
    var secDeg = s * 6;
    var hourEl = document.getElementById("clock-hour");
    var minEl = document.getElementById("clock-minute");
    var secEl = document.getElementById("clock-second");
    if (hourEl) hourEl.style.transform = "rotate(" + hourDeg + "deg)";
    if (minEl) minEl.style.transform = "rotate(" + minDeg + "deg)";
    if (secEl) secEl.style.transform = "rotate(" + secDeg + "deg)";

    var digitalEl = document.getElementById("widget-clock-digital");
    if (digitalEl) {
      var hrs = now.getHours();
      var ampm = hrs >= 12 ? "PM" : "AM";
      hrs = hrs % 12 || 12;
      digitalEl.textContent = hrs + ":" + String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0") + " " + ampm;
    }
  }
  updateWidgetClock();
  setInterval(updateWidgetClock, 1000);

  // Calendar widget
  (function () {
    var now = new Date();
    var headerEl = document.getElementById("widget-cal-header");
    var gridEl = document.getElementById("widget-cal-grid");
    if (!headerEl || !gridEl) return;

    var monthNames = ["January", "February", "March", "April", "May", "June",
      "July", "August", "September", "October", "November", "December"];
    headerEl.textContent = monthNames[now.getMonth()] + " " + now.getFullYear();

    var dayNames = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
    dayNames.forEach(function (d) {
      var span = document.createElement("span");
      span.className = "cal-day-name";
      span.textContent = d;
      gridEl.appendChild(span);
    });

    var firstDay = new Date(now.getFullYear(), now.getMonth(), 1).getDay();
    var daysInMonth = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
    var prevDays = new Date(now.getFullYear(), now.getMonth(), 0).getDate();

    for (var i = firstDay - 1; i >= 0; i--) {
      var span = document.createElement("span");
      span.className = "cal-day other-month";
      span.textContent = prevDays - i;
      gridEl.appendChild(span);
    }
    for (var d = 1; d <= daysInMonth; d++) {
      var span = document.createElement("span");
      span.className = "cal-day" + (d === now.getDate() ? " today" : "");
      span.textContent = d;
      gridEl.appendChild(span);
    }
    var remaining = 42 - firstDay - daysInMonth;
    for (var i = 1; i <= remaining; i++) {
      var span = document.createElement("span");
      span.className = "cal-day other-month";
      span.textContent = i;
      gridEl.appendChild(span);
    }
  })();

  // Weather forecast
  (function () {
    var forecastEl = document.getElementById("widget-weather-forecast");
    if (!forecastEl) return;
    var days = ["Mon", "Tue", "Wed", "Thu", "Fri"];
    var temps = ["16°", "18°", "20°", "17°", "15°"];
    var icons = ["☁️", "⛅", "☀️", "🌧️", "☁️"];
    days.forEach(function (day, i) {
      var item = document.createElement("div");
      item.className = "widget-forecast-item";
      item.innerHTML = "<div>" + day + "</div><div>" + icons[i] + "</div><div class='forecast-temp'>" + temps[i] + "</div>";
      forecastEl.appendChild(item);
    });
  })();

  // Animate system monitor values
  setInterval(function () {
    var ids = [
      { bar: "sys-cpu", pct: "sys-cpu-pct", min: 15, max: 85 },
      { bar: "sys-mem", pct: "sys-mem-pct", min: 45, max: 80 },
      { bar: "sys-net", pct: "sys-net-pct", min: 5, max: 60 },
    ];
    ids.forEach(function (item) {
      var val = item.min + Math.floor(Math.random() * (item.max - item.min));
      var barEl = document.getElementById(item.bar);
      var pctEl = document.getElementById(item.pct);
      if (barEl) barEl.style.width = val + "%";
      if (pctEl) pctEl.textContent = val + "%";
    });
  }, 3000);

  /* ══════════════════════════════════════════════════════
     9. HOT CORNERS (Issue #15)
     ══════════════════════════════════════════════════════ */
  var hotCornerTimers = {};
  var hotCornerDelay = 200;
  var missionControlOverlay = document.getElementById("mission-control-overlay");
  var launchpadOverlay = document.getElementById("launchpad-overlay");
  var hotIndicator = document.getElementById("hot-corners-indicator");

  var hotCornerActions = {
    "mission-control": function () { toggleMissionControl(); },
    "widgets": function () { widgetsPanel.classList.toggle("visible"); },
    "desktop": function () { toggleShowDesktop(); },
    "launchpad": function () { toggleLaunchpad(); },
  };

  function toggleMissionControl() {
    missionControlOverlay.classList.toggle("visible");
    launchpadOverlay.classList.remove("visible");
    if (missionControlOverlay.classList.contains("visible")) {
      var mcWins = document.getElementById("mc-windows");
      mcWins.innerHTML = "";
      windows.forEach(function (win) {
        if (win.style.display === "none") return;
        var thumb = document.createElement("div");
        thumb.className = "mc-window-thumb";
        var titleEl = win.querySelector(".window-title");
        thumb.textContent = titleEl ? titleEl.textContent : "Window";
        thumb.addEventListener("click", function () {
          focusWindow(win);
          missionControlOverlay.classList.remove("visible");
        });
        mcWins.appendChild(thumb);
      });
    }
  }

  function toggleShowDesktop() {
    var allHidden = true;
    windows.forEach(function (w) {
      if (w.style.display !== "none") allHidden = false;
    });
    if (allHidden) {
      windows.forEach(function (w) {
        w.style.display = "";
        w.style.opacity = "1";
        w.style.transform = "";
      });
    } else {
      windows.forEach(function (w) {
        w.style.transition = "opacity 0.25s ease, transform 0.25s ease";
        w.style.opacity = "0";
        w.style.transform = "scale(0.95)";
        setTimeout(function () { w.style.display = "none"; }, 260);
      });
    }
  }

  function toggleLaunchpad() {
    launchpadOverlay.classList.toggle("visible");
    missionControlOverlay.classList.remove("visible");
    if (launchpadOverlay.classList.contains("visible")) {
      var grid = document.getElementById("launchpad-grid");
      if (grid.children.length > 0) return;
      var apps = [
        { name: "Finder", color: "#007aff", icon: "📁" },
        { name: "Safari", color: "#56c0f0", icon: "🧭" },
        { name: "Messages", color: "#34c759", icon: "💬" },
        { name: "Mail", color: "#007aff", icon: "✉️" },
        { name: "Maps", color: "#32a852", icon: "🗺️" },
        { name: "Photos", color: "#f5f5f7", icon: "🖼️" },
        { name: "FaceTime", color: "#28a745", icon: "📹" },
        { name: "Calendar", color: "#ff3b30", icon: "📅" },
        { name: "Notes", color: "#ffd60a", icon: "📝" },
        { name: "Music", color: "#fc3c44", icon: "🎵" },
        { name: "TV", color: "#1d1d1f", icon: "📺" },
        { name: "App Store", color: "#0071e3", icon: "🏪" },
        { name: "Settings", color: "#636366", icon: "⚙️" },
        { name: "Color Picker", color: "#af52de", icon: "🎨" },
        { name: "Calculator", color: "#333", icon: "🔢" },
        { name: "Terminal", color: "#1d1d1f", icon: "💻" },
        { name: "TextEdit", color: "#007aff", icon: "📄" },
        { name: "Preview", color: "#5ac8fa", icon: "👁️" },
        { name: "Activity Monitor", color: "#34c759", icon: "📊" },
        { name: "Disk Utility", color: "#48c6ef", icon: "💿" },
        { name: "Console", color: "#636366", icon: "🖥️" },
      ];
      apps.forEach(function (app) {
        var el = document.createElement("div");
        el.className = "launchpad-icon";
        el.innerHTML = '<div class="launchpad-icon-img" style="background:' + app.color + ';">' + app.icon + '</div><div class="launchpad-icon-label">' + app.name + '</div>';
        el.addEventListener("click", function () {
          launchpadOverlay.classList.remove("visible");
          if (app.name === "Color Picker") {
            var cpWin = document.querySelector(".window-colorpicker");
            if (cpWin) { cpWin.style.display = ""; cpWin.style.opacity = "1"; cpWin.style.transform = ""; focusWindow(cpWin); }
          }
        });
        grid.appendChild(el);
      });
    }
  }

  // Hot corner mouse detection
  document.querySelectorAll(".hot-corner").forEach(function (corner) {
    corner.addEventListener("mouseenter", function () {
      var action = corner.dataset.action;
      hotCornerTimers[action] = setTimeout(function () {
        if (hotCornerActions[action]) hotCornerActions[action]();
      }, hotCornerDelay);
    });
    corner.addEventListener("mouseleave", function () {
      var action = corner.dataset.action;
      clearTimeout(hotCornerTimers[action]);
    });
  });

  // Close overlays on Escape
  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape") {
      missionControlOverlay.classList.remove("visible");
      launchpadOverlay.classList.remove("visible");
      widgetsPanel.classList.remove("visible");
    }
  });

  // Close overlays on click
  missionControlOverlay.addEventListener("click", function (e) {
    if (e.target === missionControlOverlay) missionControlOverlay.classList.remove("visible");
  });
  launchpadOverlay.addEventListener("click", function (e) {
    if (e.target === launchpadOverlay) launchpadOverlay.classList.remove("visible");
  });

  /* ══════════════════════════════════════════════════════
     10. COLOR PICKER (Issue #45)
     ══════════════════════════════════════════════════════ */
  (function () {
    var canvas = document.getElementById("cp-canvas");
    var ctx = canvas ? canvas.getContext("2d") : null;
    var hueSlider = document.getElementById("cp-hue");
    var preview = document.getElementById("cp-preview");
    var hexInput = document.getElementById("cp-hex");
    var rInput = document.getElementById("cp-r");
    var gInput = document.getElementById("cp-g");
    var bInput = document.getElementById("cp-b");
    var saveBtn = document.getElementById("cp-save-btn");
    var palette = document.getElementById("cp-palette");

    if (!canvas || !ctx) return;

    var currentHue = 11;
    var currentR = 255, currentG = 87, currentB = 51;

    function drawColorField(hue) {
      var w = canvas.width, h = canvas.height;
      // Draw saturation-brightness plane
      for (var x = 0; x < w; x++) {
        var satGrad = ctx.createLinearGradient(0, 0, 0, h);
        var hsl = "hsl(" + hue + ", " + Math.round((x / w) * 100) + "%, 50%)";
        satGrad.addColorStop(0, "#fff");
        satGrad.addColorStop(0.5, hsl);
        satGrad.addColorStop(1, "#000");
        ctx.fillStyle = satGrad;
        ctx.fillRect(x, 0, 1, h);
      }
    }

    function updateFromRGB(r, g, b) {
      currentR = r; currentG = g; currentB = b;
      var hex = "#" + [r, g, b].map(function (v) { return v.toString(16).padStart(2, "0"); }).join("").toUpperCase();
      if (preview) preview.style.background = hex;
      if (hexInput) hexInput.value = hex;
      if (rInput) rInput.value = r;
      if (gInput) gInput.value = g;
      if (bInput) bInput.value = b;
    }

    function hexToRGB(hex) {
      hex = hex.replace("#", "");
      if (hex.length === 3) hex = hex[0]+hex[0]+hex[1]+hex[1]+hex[2]+hex[2];
      return {
        r: parseInt(hex.substring(0, 2), 16),
        g: parseInt(hex.substring(2, 4), 16),
        b: parseInt(hex.substring(4, 6), 16),
      };
    }

    drawColorField(currentHue);

    if (hueSlider) {
      hueSlider.addEventListener("input", function () {
        currentHue = parseInt(this.value);
        drawColorField(currentHue);
      });
    }

    canvas.addEventListener("click", function (e) {
      var rect = canvas.getBoundingClientRect();
      var x = Math.round((e.clientX - rect.left) * (canvas.width / rect.width));
      var y = Math.round((e.clientY - rect.top) * (canvas.height / rect.height));
      var pixel = ctx.getImageData(x, y, 1, 1).data;
      updateFromRGB(pixel[0], pixel[1], pixel[2]);
    });

    // Dragging on canvas
    var cpDragging = false;
    canvas.addEventListener("mousedown", function (e) {
      cpDragging = true;
      var rect = canvas.getBoundingClientRect();
      var x = Math.round((e.clientX - rect.left) * (canvas.width / rect.width));
      var y = Math.round((e.clientY - rect.top) * (canvas.height / rect.height));
      var pixel = ctx.getImageData(x, y, 1, 1).data;
      updateFromRGB(pixel[0], pixel[1], pixel[2]);
    });
    document.addEventListener("mousemove", function (e) {
      if (!cpDragging) return;
      var rect = canvas.getBoundingClientRect();
      var x = Math.max(0, Math.min(canvas.width - 1, Math.round((e.clientX - rect.left) * (canvas.width / rect.width))));
      var y = Math.max(0, Math.min(canvas.height - 1, Math.round((e.clientY - rect.top) * (canvas.height / rect.height))));
      var pixel = ctx.getImageData(x, y, 1, 1).data;
      updateFromRGB(pixel[0], pixel[1], pixel[2]);
    });
    document.addEventListener("mouseup", function () { cpDragging = false; });

    // Update from hex input
    if (hexInput) {
      hexInput.addEventListener("change", function () {
        var val = this.value.trim();
        if (/^#?[0-9a-fA-F]{3,6}$/.test(val)) {
          var rgb = hexToRGB(val);
          updateFromRGB(rgb.r, rgb.g, rgb.b);
        }
      });
    }

    // Update from RGB inputs
    [rInput, gInput, bInput].forEach(function (input) {
      if (!input) return;
      input.addEventListener("change", function () {
        var r = parseInt(rInput.value) || 0;
        var g = parseInt(gInput.value) || 0;
        var b = parseInt(bInput.value) || 0;
        updateFromRGB(
          Math.max(0, Math.min(255, r)),
          Math.max(0, Math.min(255, g)),
          Math.max(0, Math.min(255, b))
        );
      });
    });

    // Save to palette
    if (saveBtn) {
      saveBtn.addEventListener("click", function () {
        var hex = hexInput.value;
        var swatch = document.createElement("div");
        swatch.className = "cp-swatch";
        swatch.style.background = hex;
        swatch.dataset.color = hex;
        swatch.addEventListener("click", function () {
          var rgb = hexToRGB(swatch.dataset.color);
          updateFromRGB(rgb.r, rgb.g, rgb.b);
        });
        palette.insertBefore(swatch, saveBtn);
      });
    }

    // Palette swatches click
    document.querySelectorAll(".cp-swatch[data-color]").forEach(function (sw) {
      sw.addEventListener("click", function () {
        var rgb = hexToRGB(sw.dataset.color);
        updateFromRGB(rgb.r, rgb.g, rgb.b);
      });
    });
  })();

  // Open Color Picker from dock
  var colorPickerDockIcon = document.querySelector('[data-app="System Preferences"]');
  // We'll use a dedicated dock icon approach: open via Launchpad

  /* ══════════════════════════════════════════════════════
     11. PICTURE-IN-PICTURE (Issue #63)
     ══════════════════════════════════════════════════════ */
  (function () {
    var pipWin = document.getElementById("pip-window");
    var pipClose = document.getElementById("pip-close");
    var pipBack = document.getElementById("pip-back");
    var pipTitlebar = document.getElementById("pip-titlebar");
    var pipPlayBtn = document.getElementById("pip-play-btn");
    var pipResize = document.getElementById("pip-resize");

    if (!pipWin) return;

    // Drag PiP
    var pipDragging = false;
    var pipOffX = 0, pipOffY = 0;
    pipTitlebar.addEventListener("mousedown", function (e) {
      if (e.target.closest(".pip-btn")) return;
      pipDragging = true;
      pipOffX = e.clientX - pipWin.offsetLeft;
      pipOffY = e.clientY - pipWin.offsetTop;
      e.preventDefault();
    });
    document.addEventListener("mousemove", function (e) {
      if (!pipDragging) return;
      pipWin.style.left = Math.max(0, e.clientX - pipOffX) + "px";
      pipWin.style.top = Math.max(0, e.clientY - pipOffY) + "px";
      pipWin.style.right = "auto";
      pipWin.style.bottom = "auto";
    });
    document.addEventListener("mouseup", function () { pipDragging = false; });

    // Close PiP
    pipClose.addEventListener("click", function () {
      pipWin.style.display = "none";
    });

    // Back to app
    pipBack.addEventListener("click", function () {
      pipWin.style.display = "none";
    });

    // Play/Pause toggle
    var isPlaying = true;
    pipPlayBtn.addEventListener("click", function () {
      isPlaying = !isPlaying;
      pipPlayBtn.textContent = isPlaying ? "▶" : "⏸";
      pipPlayBtn.classList.toggle("playing", !isPlaying);
      var waves = pipWin.querySelectorAll(".pip-wave");
      waves.forEach(function (w) {
        w.style.animationPlayState = isPlaying ? "running" : "paused";
      });
    });

    // Resize PiP (constrain 16:9)
    var pipResizing = false;
    var pipStartW = 0, pipStartH = 0, pipStartX = 0, pipStartY = 0;
    pipResize.addEventListener("mousedown", function (e) {
      pipResizing = true;
      pipStartW = pipWin.offsetWidth;
      pipStartH = pipWin.offsetHeight;
      pipStartX = e.clientX;
      pipStartY = e.clientY;
      e.preventDefault();
      e.stopPropagation();
    });
    document.addEventListener("mousemove", function (e) {
      if (!pipResizing) return;
      var dw = e.clientX - pipStartX;
      var newW = Math.max(200, Math.min(640, pipStartW + dw));
      var newH = Math.round(newW * 9 / 16);
      pipWin.style.width = newW + "px";
      pipWin.style.height = newH + "px";
    });
    document.addEventListener("mouseup", function () { pipResizing = false; });

    // Double-click to toggle size
    pipWin.addEventListener("dblclick", function (e) {
      if (e.target.closest(".pip-btn") || e.target.closest(".pip-play-btn")) return;
      var currW = pipWin.offsetWidth;
      if (currW < 400) {
        pipWin.style.width = "480px";
        pipWin.style.height = "270px";
      } else {
        pipWin.style.width = "320px";
        pipWin.style.height = "180px";
      }
    });

    // Show PiP when TV dock icon is clicked
    var tvIcon = document.querySelector('[data-app="TV"]');
    if (tvIcon) {
      tvIcon.addEventListener("click", function () {
        pipWin.style.display = "";
      });
    }
  })();

  /* ══════════════════════════════════════════════════════
     12. WINDOW TAB SUPPORT (Issue #75)
     ══════════════════════════════════════════════════════ */
  (function () {
    // Add tabs to applicable windows: Safari, Maps, Messages
    var tabbableWindows = [
      { selector: ".window-safari", name: "Safari", tabs: ["apple.com", "google.com", "github.com"] },
      { selector: ".window-maps", name: "Maps", tabs: ["San Francisco", "New York"] },
    ];

    tabbableWindows.forEach(function (config) {
      var win = document.querySelector(config.selector);
      if (!win) return;

      var titlebar = win.querySelector(".window-titlebar");
      var content = win.querySelector(".window-content");
      if (!titlebar || !content) return;

      // Create tab bar
      var tabBar = document.createElement("div");
      tabBar.className = "window-tabs";

      // Store original content
      var originalHTML = content.innerHTML;

      // Create tabs
      config.tabs.forEach(function (tabName, idx) {
        var tab = document.createElement("div");
        tab.className = "window-tab" + (idx === 0 ? " active" : "");
        tab.innerHTML = '<span class="window-tab-title">' + tabName + '</span><span class="window-tab-close">✕</span>';
        tab.dataset.tabIndex = idx;

        tab.addEventListener("click", function (e) {
          if (e.target.classList.contains("window-tab-close")) {
            // Close tab
            tab.remove();
            var remaining = tabBar.querySelectorAll(".window-tab");
            if (remaining.length === 0) {
              // Remove tab bar if no tabs left
              tabBar.remove();
              return;
            }
            if (tab.classList.contains("active") && remaining.length > 0) {
              remaining[0].classList.add("active");
            }
            return;
          }
          // Switch tab
          tabBar.querySelectorAll(".window-tab").forEach(function (t) { t.classList.remove("active"); });
          tab.classList.add("active");
          // Update window title
          var titleEl = win.querySelector(".window-title");
          if (titleEl) titleEl.textContent = tabName;
        });

        tabBar.appendChild(tab);
      });

      // Add new tab button
      var addTab = document.createElement("div");
      addTab.className = "window-tab-add";
      addTab.textContent = "+";
      addTab.title = "New Tab (Ctrl+T)";
      addTab.addEventListener("click", function () {
        var tabCount = tabBar.querySelectorAll(".window-tab").length;
        var newName = config.name + " Tab " + (tabCount + 1);
        var tab = document.createElement("div");
        tab.className = "window-tab active";
        tab.innerHTML = '<span class="window-tab-title">' + newName + '</span><span class="window-tab-close">✕</span>';

        tabBar.querySelectorAll(".window-tab").forEach(function (t) { t.classList.remove("active"); });

        tab.addEventListener("click", function (e) {
          if (e.target.classList.contains("window-tab-close")) {
            tab.remove();
            var remaining = tabBar.querySelectorAll(".window-tab");
            if (remaining.length > 0 && tab.classList.contains("active")) {
              remaining[0].classList.add("active");
            }
            return;
          }
          tabBar.querySelectorAll(".window-tab").forEach(function (t) { t.classList.remove("active"); });
          tab.classList.add("active");
          var titleEl = win.querySelector(".window-title");
          if (titleEl) titleEl.textContent = newName;
        });

        tabBar.insertBefore(tab, addTab);
      });
      tabBar.appendChild(addTab);

      // Insert tab bar after titlebar
      titlebar.insertAdjacentElement("afterend", tabBar);
    });

    // Keyboard shortcuts for tabs
    document.addEventListener("keydown", function (e) {
      // Ctrl+T: New tab in focused window
      if (e.ctrlKey && e.key === "t") {
        e.preventDefault();
        var focused = document.querySelector(".window.focused .window-tab-add");
        if (focused) focused.click();
      }
      // Ctrl+W: Close current tab
      if (e.ctrlKey && e.key === "w") {
        e.preventDefault();
        var activeTab = document.querySelector(".window.focused .window-tab.active .window-tab-close");
        if (activeTab) activeTab.click();
      }
      // Ctrl+Tab: Next tab
      if (e.ctrlKey && e.key === "Tab") {
        e.preventDefault();
        var focusedWin = document.querySelector(".window.focused");
        if (!focusedWin) return;
        var tabs = focusedWin.querySelectorAll(".window-tab");
        var activeIdx = -1;
        tabs.forEach(function (t, i) { if (t.classList.contains("active")) activeIdx = i; });
        if (activeIdx >= 0 && tabs.length > 1) {
          var nextIdx = e.shiftKey ?
            (activeIdx - 1 + tabs.length) % tabs.length :
            (activeIdx + 1) % tabs.length;
          tabs[activeIdx].classList.remove("active");
          tabs[nextIdx].classList.add("active");
          var titleEl = focusedWin.querySelector(".window-title");
          var tabTitle = tabs[nextIdx].querySelector(".window-tab-title");
          if (titleEl && tabTitle) titleEl.textContent = tabTitle.textContent;
        }
      }
    });
  })();

  /* ══════════════════════════════════════════════════════
     13. CLIPBOARD HISTORY (Issue #13)
     ══════════════════════════════════════════════════════ */
  (function () {
    var clipPanel = document.getElementById("clipboard-panel");
    var clipList = document.getElementById("clipboard-list");
    var clipSearchInput = document.getElementById("clipboard-search-input");
    var clipClearBtn = document.getElementById("clipboard-clear-btn");
    if (!clipPanel) return;

    // Inline clipboard history logic (mirrors module)
    var clipHistory = [];
    var maxClip = 50;

    function clipCopy(content, type, source) {
      if (!content) return;
      var idx = clipHistory.findIndex(function (e) { return e.content === content; });
      if (idx !== -1) clipHistory.splice(idx, 1);
      clipHistory.unshift({
        content: content, type: type || "text", source: source || "",
        timestamp: Date.now(), pinned: false,
      });
      while (clipHistory.length > maxClip) {
        for (var i = clipHistory.length - 1; i >= 0; i--) {
          if (!clipHistory[i].pinned) { clipHistory.splice(i, 1); break; }
        }
      }
    }

    // Intercept Ctrl+C
    document.addEventListener("copy", function () {
      var sel = window.getSelection().toString();
      if (sel) clipCopy(sel, "text", "Selection");
    });

    function renderClipboard(filter) {
      clipList.innerHTML = "";
      var items = clipHistory;
      if (filter) {
        var q = filter.toLowerCase();
        items = items.filter(function (e) { return e.content.toLowerCase().indexOf(q) !== -1; });
      }
      if (items.length === 0) {
        clipList.innerHTML = '<div class="clipboard-empty">' + (filter ? "No matches" : "No clipboard history yet") + "</div>";
        return;
      }
      items.forEach(function (entry, i) {
        var el = document.createElement("div");
        el.className = "clipboard-item";
        var age = Math.round((Date.now() - entry.timestamp) / 60000);
        var ageStr = age < 1 ? "now" : age + "m ago";
        el.innerHTML =
          '<span class="clipboard-item-pin ' + (entry.pinned ? "pinned" : "") + '" data-idx="' + i + '">📌</span>' +
          '<div class="clipboard-item-content">' + escapeHtml(entry.content) + "</div>" +
          '<span class="clipboard-item-meta">' + ageStr + "</span>";
        el.addEventListener("click", function (e) {
          if (e.target.classList.contains("clipboard-item-pin")) {
            entry.pinned = !entry.pinned;
            renderClipboard(filter);
            return;
          }
          // Copy to clipboard
          navigator.clipboard.writeText(entry.content).catch(function () {});
          clipPanel.classList.remove("visible");
        });
        clipList.appendChild(el);
      });
    }

    function escapeHtml(s) {
      var d = document.createElement("div");
      d.textContent = s;
      return d.innerHTML;
    }

    // Toggle clipboard panel: Ctrl+Shift+V
    document.addEventListener("keydown", function (e) {
      if (e.ctrlKey && e.shiftKey && e.key === "V") {
        e.preventDefault();
        clipPanel.classList.toggle("visible");
        if (clipPanel.classList.contains("visible")) {
          renderClipboard();
          setTimeout(function () { clipSearchInput.focus(); }, 50);
        }
      }
    });

    clipSearchInput.addEventListener("input", function () {
      renderClipboard(this.value);
    });

    clipClearBtn.addEventListener("click", function () {
      clipHistory = clipHistory.filter(function (e) { return e.pinned; });
      renderClipboard();
    });

    // Close on outside click
    document.addEventListener("click", function (e) {
      if (clipPanel.classList.contains("visible") && !clipPanel.contains(e.target)) {
        clipPanel.classList.remove("visible");
      }
    });

    // Seed some demo entries
    clipCopy("Hello, world!", "text", "TextEdit");
    clipCopy("https://auroraos.dev", "link", "Safari");
    clipCopy("npm install auroraos-sdk", "text", "Terminal");
  })();

  /* ══════════════════════════════════════════════════════
     14. NOTIFICATION CENTER (Issue #52)
     ══════════════════════════════════════════════════════ */
  (function () {
    var notifPanel = document.getElementById("notif-panel");
    var notifList = document.getElementById("notif-list");
    var notifBadge = document.getElementById("notif-badge");
    var notifClearAll = document.getElementById("notif-clear-all");
    var notifDndBtn = document.getElementById("notif-dnd-btn");
    if (!notifPanel) return;

    var notifications = [];
    var nextId = 1;
    var dnd = false;

    function addNotif(app, icon, title, body, actions) {
      notifications.unshift({
        id: nextId++, app: app, icon: icon, title: title,
        body: body, actions: actions || [], read: false, timestamp: Date.now(),
      });
      updateBadge();
      renderNotifs();
    }

    function updateBadge() {
      var unread = notifications.filter(function (n) { return !n.read; }).length;
      if (unread > 0) {
        notifBadge.style.display = "";
        notifBadge.textContent = unread;
      } else {
        notifBadge.style.display = "none";
      }
    }

    function timeAgo(ts) {
      var m = Math.round((Date.now() - ts) / 60000);
      if (m < 1) return "now";
      if (m < 60) return m + "m";
      return Math.round(m / 60) + "h";
    }

    function renderNotifs() {
      notifList.innerHTML = "";
      if (notifications.length === 0) {
        notifList.innerHTML = '<div class="notif-empty">No new notifications</div>';
        return;
      }
      // Group by app
      var groups = {};
      notifications.forEach(function (n) {
        if (!groups[n.app]) groups[n.app] = [];
        groups[n.app].push(n);
      });
      Object.keys(groups).forEach(function (app) {
        var header = document.createElement("div");
        header.className = "notif-group-header";
        header.innerHTML = "<span>" + app + " (" + groups[app].length + ")</span>";
        var clearGrp = document.createElement("button");
        clearGrp.className = "notif-group-clear";
        clearGrp.textContent = "Clear";
        clearGrp.addEventListener("click", function () {
          notifications = notifications.filter(function (n) { return n.app !== app; });
          updateBadge();
          renderNotifs();
        });
        header.appendChild(clearGrp);
        notifList.appendChild(header);

        groups[app].forEach(function (n) {
          var card = document.createElement("div");
          card.className = "notif-card";
          var actionsHtml = "";
          if (n.actions.length) {
            actionsHtml = '<div class="notif-card-actions">' +
              n.actions.map(function (a) { return '<button class="notif-action-btn" data-action="' + a + '">' + a + '</button>'; }).join("") +
              "</div>";
          }
          card.innerHTML =
            '<div class="notif-card-top">' +
              '<span class="notif-card-icon">' + n.icon + '</span>' +
              '<div class="notif-card-body"><div class="notif-card-title">' + n.title + '</div><div class="notif-card-text">' + n.body + '</div></div>' +
              '<span class="notif-card-time">' + timeAgo(n.timestamp) + '</span>' +
            '</div>' + actionsHtml;
          card.addEventListener("click", function (e) {
            if (e.target.classList.contains("notif-action-btn")) {
              n.read = true;
              updateBadge();
              renderNotifs();
              return;
            }
            n.read = true;
            updateBadge();
          });
          notifList.appendChild(card);
        });
      });
    }

    // Open notification panel from bell area (reuse cc-toggle area)
    // We'll use a dedicated button — add click to the clock to also toggle notifs
    var notifToggle = document.getElementById("cc-toggle");
    notifToggle.addEventListener("dblclick", function (e) {
      e.stopPropagation();
      notifPanel.classList.toggle("visible");
      widgetsPanel.classList.remove("visible");
    });

    notifClearAll.addEventListener("click", function () {
      notifications = [];
      updateBadge();
      renderNotifs();
    });

    notifDndBtn.addEventListener("click", function () {
      dnd = !dnd;
      notifDndBtn.classList.toggle("active", dnd);
    });

    // Close on outside click
    document.addEventListener("click", function (e) {
      if (notifPanel.classList.contains("visible") &&
          !notifPanel.contains(e.target) && e.target !== notifToggle) {
        notifPanel.classList.remove("visible");
      }
    });

    // Seed demo notifications
    setTimeout(function () {
      addNotif("Messages", "💬", "John Appleseed", "Hey! The new build looks great", ["Reply", "Mark as Read"]);
      addNotif("Mail", "✉️", "Weekly Report", "Your weekly summary is ready to view", ["Open", "Archive"]);
      addNotif("Calendar", "📅", "Team Standup", "Starting in 15 minutes", ["Join", "Snooze"]);
      addNotif("Messages", "💬", "Sarah Connor", "Can you review the PR?", ["Reply"]);
      addNotif("App Store", "🏪", "Update Available", "AuroraOS 2.1 is ready to install", ["Update"]);
    }, 500);

    renderNotifs();
  })();

  /* ══════════════════════════════════════════════════════
     15. SCREENSHOT TOOL (Issue #12)
     ══════════════════════════════════════════════════════ */
  (function () {
    var ssOverlay = document.getElementById("screenshot-overlay");
    var ssToolbar = document.getElementById("screenshot-toolbar");
    var ssCaptureBtn = document.getElementById("ss-capture-btn");
    var ssCancelBtn = document.getElementById("ss-cancel-btn");
    var ssSelection = document.getElementById("screenshot-selection");
    var ssThumb = document.getElementById("screenshot-thumb");
    if (!ssOverlay) return;

    var ssMode = "fullscreen";
    var ssSelecting = false;
    var ssStart = { x: 0, y: 0 };

    // Mode buttons
    document.querySelectorAll("[data-ss-mode]").forEach(function (btn) {
      btn.addEventListener("click", function (e) {
        e.stopPropagation();
        document.querySelectorAll("[data-ss-mode]").forEach(function (b) { b.classList.remove("ss-mode-active"); });
        btn.classList.add("ss-mode-active");
        ssMode = btn.dataset.ssMode;
      });
    });

    // Open screenshot tool: Ctrl+Shift+5 (macOS-like combo)
    document.addEventListener("keydown", function (e) {
      if (e.ctrlKey && e.shiftKey && e.key === "5") {
        e.preventDefault();
        ssOverlay.classList.toggle("visible");
      }
      // Quick fullscreen: Ctrl+Shift+3
      if (e.ctrlKey && e.shiftKey && e.key === "3") {
        e.preventDefault();
        doCapture("fullscreen");
      }
      // Quick selection: Ctrl+Shift+4
      if (e.ctrlKey && e.shiftKey && e.key === "4") {
        e.preventDefault();
        ssMode = "selection";
        ssOverlay.classList.add("visible");
        document.querySelectorAll("[data-ss-mode]").forEach(function (b) { b.classList.remove("ss-mode-active"); });
        var selBtn = document.querySelector('[data-ss-mode="selection"]');
        if (selBtn) selBtn.classList.add("ss-mode-active");
      }
    });

    // Selection drag
    ssOverlay.addEventListener("mousedown", function (e) {
      if (ssMode !== "selection") return;
      if (ssToolbar.contains(e.target)) return;
      ssSelecting = true;
      ssStart.x = e.clientX;
      ssStart.y = e.clientY;
      ssSelection.style.display = "block";
      ssSelection.style.left = e.clientX + "px";
      ssSelection.style.top = e.clientY + "px";
      ssSelection.style.width = "0";
      ssSelection.style.height = "0";
    });
    ssOverlay.addEventListener("mousemove", function (e) {
      if (!ssSelecting) return;
      var x = Math.min(e.clientX, ssStart.x);
      var y = Math.min(e.clientY, ssStart.y);
      var w = Math.abs(e.clientX - ssStart.x);
      var h = Math.abs(e.clientY - ssStart.y);
      ssSelection.style.left = x + "px";
      ssSelection.style.top = y + "px";
      ssSelection.style.width = w + "px";
      ssSelection.style.height = h + "px";
      // Show dimensions
      var dims = ssSelection.querySelector(".ss-dims");
      if (!dims) {
        dims = document.createElement("div");
        dims.className = "ss-dims";
        ssSelection.appendChild(dims);
      }
      dims.textContent = w + " × " + h;
    });
    ssOverlay.addEventListener("mouseup", function () {
      if (ssSelecting) {
        ssSelecting = false;
        doCapture("selection");
      }
    });

    // Capture button
    ssCaptureBtn.addEventListener("click", function (e) {
      e.stopPropagation();
      doCapture(ssMode);
    });

    ssCancelBtn.addEventListener("click", function (e) {
      e.stopPropagation();
      ssOverlay.classList.remove("visible");
      ssSelection.style.display = "none";
    });

    function doCapture(mode) {
      ssOverlay.classList.remove("visible");
      ssSelection.style.display = "none";

      // Flash
      var flash = document.createElement("div");
      flash.className = "screenshot-flash";
      document.body.appendChild(flash);
      setTimeout(function () { flash.remove(); }, 500);

      // Show thumbnail
      ssThumb.classList.add("visible");
      setTimeout(function () { ssThumb.classList.remove("visible"); }, 4000);
    }
  })();

  /* ══════════════════════════════════════════════════════
     16. GLOBAL UNDO/REDO (Issue #65)
     ══════════════════════════════════════════════════════ */
  (function () {
    // Per-window undo stacks
    var undoStacks = {};

    function getStack(winId) {
      if (!undoStacks[winId]) {
        undoStacks[winId] = { undo: [], redo: [] };
      }
      return undoStacks[winId];
    }

    function pushUndo(winId, action) {
      var s = getStack(winId);
      s.undo.push(action);
      s.redo = [];
      if (s.undo.length > 100) s.undo.shift();
    }

    // Ctrl+Z / Ctrl+Y
    document.addEventListener("keydown", function (e) {
      if (e.ctrlKey && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        var focused = document.querySelector(".window.focused");
        if (!focused) return;
        var winId = focused.dataset.window || "default";
        var s = getStack(winId);
        if (s.undo.length > 0) {
          var action = s.undo.pop();
          s.redo.push(action);
          if (action.undo) action.undo();
        }
      }
      if (e.ctrlKey && (e.key === "y" || (e.shiftKey && e.key === "Z"))) {
        e.preventDefault();
        var focused = document.querySelector(".window.focused");
        if (!focused) return;
        var winId = focused.dataset.window || "default";
        var s = getStack(winId);
        if (s.redo.length > 0) {
          var action = s.redo.pop();
          s.undo.push(action);
          if (action.redo) action.redo();
        }
      }
    });

    // Update Edit menu to show Undo/Redo state
    var editMenu = menuData["Edit"];
    if (editMenu) {
      menuData["Edit"] = ["Undo  ⌘Z", "Redo  ⌘⇧Z", "---", "Cut", "Copy", "Paste", "Select All"];
    }

    // Make undo available globally
    window._auroraUndo = { push: pushUndo, getStack: getStack };
  })();

  /* ══════════════════════════════════════════════════════
     17. SHARE SHEET (Issue #56)
     ══════════════════════════════════════════════════════ */
  (function () {
    var shareOverlay = document.getElementById("share-overlay");
    var shareSheet = document.getElementById("share-sheet");
    var shareCloseBtn = document.getElementById("share-close-btn");
    var shareTargets = document.getElementById("share-targets");
    var sharePreview = document.getElementById("share-preview");
    if (!shareOverlay) return;

    var targets = [
      { name: "AirDrop", icon: "📡", bg: "#007aff" },
      { name: "Messages", icon: "💬", bg: "#34c759" },
      { name: "Mail", icon: "✉️", bg: "#007aff" },
      { name: "Notes", icon: "📝", bg: "#ffd60a" },
      { name: "Reminders", icon: "☑️", bg: "#ff9500" },
      { name: "Photos", icon: "🖼️", bg: "#ff2d55" },
      { name: "Twitter", icon: "🐦", bg: "#1da1f2" },
      { name: "Facebook", icon: "📘", bg: "#1877f2" },
    ];

    // Render targets
    targets.forEach(function (t) {
      var el = document.createElement("div");
      el.className = "share-target";
      el.innerHTML =
        '<div class="share-target-icon" style="background:' + t.bg + ';">' + t.icon + '</div>' +
        '<span class="share-target-label">' + t.name + '</span>';
      el.addEventListener("click", function () {
        shareOverlay.classList.remove("visible");
        // Show brief confirmation
        var indicator = document.getElementById("hot-corners-indicator");
        if (indicator) {
          indicator.textContent = "Shared to " + t.name;
          indicator.classList.add("visible");
          setTimeout(function () { indicator.classList.remove("visible"); }, 1500);
        }
      });
      shareTargets.appendChild(el);
    });

    // Open share sheet from right-click context menu
    // Add "Share…" to context menu items
    var ctxItems = document.querySelector(".desktop");
    if (ctxItems) {
      ctxItems.addEventListener("contextmenu", function () {
        setTimeout(function () {
          var menu = document.querySelector(".context-menu");
          if (menu && !menu.querySelector(".share-ctx-item")) {
            var sep = document.createElement("div");
            sep.className = "menu-dropdown-sep";
            menu.appendChild(sep);
            var shareItem = document.createElement("div");
            shareItem.className = "menu-dropdown-item share-ctx-item";
            shareItem.textContent = "Share\u2026";
            shareItem.addEventListener("click", function () {
              menu.remove();
              openShareSheet("Selected content from desktop");
            });
            menu.appendChild(shareItem);
          }
        }, 10);
      });
    }

    function openShareSheet(content) {
      sharePreview.textContent = content || "";
      shareOverlay.classList.add("visible");
    }

    shareCloseBtn.addEventListener("click", function () {
      shareOverlay.classList.remove("visible");
    });

    shareOverlay.addEventListener("click", function (e) {
      if (e.target === shareOverlay) shareOverlay.classList.remove("visible");
    });

    // Share action items
    document.querySelectorAll("[data-share-action]").forEach(function (item) {
      item.addEventListener("click", function () {
        shareOverlay.classList.remove("visible");
        var indicator = document.getElementById("hot-corners-indicator");
        if (indicator) {
          indicator.textContent = "Action completed";
          indicator.classList.add("visible");
          setTimeout(function () { indicator.classList.remove("visible"); }, 1200);
        }
      });
    });

    // Expose for external use
    window._auroraShare = { open: openShareSheet };
  })();

  /* ══════════════════════════════════════════════════════
     18. KEYCHAIN / PASSWORD MANAGER (Issue #58)
     ══════════════════════════════════════════════════════ */
  (function () {
    var kcList = document.getElementById("kc-list");
    var kcSearch = document.getElementById("kc-search");
    var kcAddBtn = document.getElementById("kc-add-btn");
    if (!kcList) return;

    var entries = [
      { id: 1, name: "GitHub", account: "user@mail.com", password: "Gh$tr0ng!Pass9", url: "github.com", category: "password" },
      { id: 2, name: "Netflix", account: "user@mail.com", password: "Nfl1x#2026", url: "netflix.com", category: "password" },
      { id: 3, name: "Apple ID", account: "user@icloud.com", password: "Ap!D_Str0nG#7", url: "apple.com", category: "password" },
      { id: 4, name: "AWS API Key", notes: "AKIA...redacted", category: "secure-note" },
      { id: 5, name: "SSH Key", account: "id_rsa", category: "key" },
      { id: 6, name: "Google", account: "user@gmail.com", password: "g00gL3!", url: "google.com", category: "password" },
    ];
    var activeCategory = "password";
    var nextId = 7;

    function evalStrength(pw) {
      if (!pw || pw.length < 6) return "weak";
      var score = 0;
      if (pw.length >= 8) score++;
      if (pw.length >= 12) score++;
      if (/[A-Z]/.test(pw)) score++;
      if (/[a-z]/.test(pw)) score++;
      if (/[0-9]/.test(pw)) score++;
      if (/[^a-zA-Z0-9]/.test(pw)) score++;
      return score <= 2 ? "weak" : score <= 4 ? "medium" : "strong";
    }

    function renderKc(filter) {
      kcList.innerHTML = "";
      var items = entries.filter(function (e) { return e.category === activeCategory; });
      if (filter) {
        var q = filter.toLowerCase();
        items = items.filter(function (e) {
          return e.name.toLowerCase().indexOf(q) !== -1 ||
                 (e.account && e.account.toLowerCase().indexOf(q) !== -1);
        });
      }
      if (items.length === 0) {
        kcList.innerHTML = '<div style="padding:20px;text-align:center;color:#999;font-size:12px;">No entries</div>';
        return;
      }
      items.forEach(function (entry) {
        var el = document.createElement("div");
        el.className = "kc-entry";
        var icon = entry.category === "password" ? "🔑" : entry.category === "secure-note" ? "📝" : entry.category === "certificate" ? "📜" : "🔐";
        var strengthHtml = "";
        if (entry.password) {
          var s = evalStrength(entry.password);
          strengthHtml = '<span class="kc-entry-strength kc-strength-' + s + '">' + s + "</span>";
        }
        el.innerHTML = '<span class="kc-entry-icon">' + icon + "</span>" +
          '<div class="kc-entry-info"><div class="kc-entry-name">' + entry.name + '</div><div class="kc-entry-account">' + (entry.account || entry.notes || "") + "</div></div>" +
          strengthHtml;
        kcList.appendChild(el);
      });
    }

    document.querySelectorAll("[data-kc-cat]").forEach(function (btn) {
      btn.addEventListener("click", function () {
        document.querySelectorAll("[data-kc-cat]").forEach(function (b) { b.classList.remove("kc-cat-active"); });
        btn.classList.add("kc-cat-active");
        activeCategory = btn.dataset.kcCat;
        renderKc(kcSearch.value);
      });
    });

    kcSearch.addEventListener("input", function () { renderKc(this.value); });

    kcAddBtn.addEventListener("click", function () {
      var name = prompt("Entry name:");
      if (!name) return;
      var account = prompt("Account/Username:") || "";
      var pw = prompt("Password (or leave empty):") || "";
      entries.push({ id: nextId++, name: name, account: account, password: pw, url: "", category: activeCategory });
      renderKc(kcSearch.value);
    });

    renderKc();
  })();

  /* ══════════════════════════════════════════════════════
     19. REMINDERS / TO-DO (Issue #29)
     ══════════════════════════════════════════════════════ */
  (function () {
    var remTaskList = document.getElementById("rem-task-list");
    var remAddTaskBtn = document.getElementById("rem-add-task-btn");
    var remSort = document.getElementById("rem-sort");
    var remLists = document.getElementById("rem-lists");
    var remAddListBtn = document.getElementById("rem-add-list-btn");
    if (!remTaskList) return;

    var lists = [{ id: 1, name: "Reminders", color: "#007aff" }];
    var tasks = [
      { id: 1, title: "Review AuroraOS design docs", notes: "", dueDate: "2026-03-21", completed: false, completedAt: null, flagged: true, priority: "high", listId: 1 },
      { id: 2, title: "Buy groceries", notes: "Milk, eggs, bread", dueDate: "2026-03-22", completed: false, completedAt: null, flagged: false, priority: "medium", listId: 1 },
      { id: 3, title: "Call dentist", notes: "", dueDate: null, completed: false, completedAt: null, flagged: false, priority: "low", listId: 1 },
      { id: 4, title: "Finish Rust book chapter 12", notes: "", dueDate: "2026-03-25", completed: true, completedAt: Date.now(), flagged: false, priority: "none", listId: 1 },
      { id: 5, title: "Ship v2.0 release", notes: "Final QA pass", dueDate: "2026-04-01", completed: false, completedAt: null, flagged: true, priority: "high", listId: 1 },
    ];
    var nextTaskId = 6;
    var nextListId = 2;
    var currentView = "all";

    function getViewTasks() {
      var today = new Date().toISOString().split("T")[0];
      if (currentView === "today") return tasks.filter(function (t) { return t.dueDate === today && !t.completed; });
      if (currentView === "scheduled") return tasks.filter(function (t) { return t.dueDate && !t.completed; });
      if (currentView === "flagged") return tasks.filter(function (t) { return t.flagged && !t.completed; });
      if (currentView === "completed") return tasks.filter(function (t) { return t.completed; });
      return tasks.filter(function (t) { return !t.completed; });
    }

    function sortTasksList(arr, by) {
      var pOrder = { high: 0, medium: 1, low: 2, none: 3 };
      if (by === "priority") arr.sort(function (a, b) { return (pOrder[a.priority] || 3) - (pOrder[b.priority] || 3); });
      else if (by === "dueDate") arr.sort(function (a, b) { return (a.dueDate || "9999").localeCompare(b.dueDate || "9999"); });
      else if (by === "title") arr.sort(function (a, b) { return a.title.localeCompare(b.title); });
      return arr;
    }

    function renderTasks() {
      remTaskList.innerHTML = "";
      var viewTasks = getViewTasks();
      var sortBy = remSort.value;
      if (sortBy !== "default") viewTasks = sortTasksList(viewTasks, sortBy);
      if (viewTasks.length === 0) {
        remTaskList.innerHTML = '<div style="padding:20px;text-align:center;color:#999;font-size:12px;">No reminders</div>';
        return;
      }
      viewTasks.forEach(function (task) {
        var el = document.createElement("div");
        el.className = "rem-task";
        var priClass = task.priority !== "none" ? " rem-priority-" + task.priority : "";
        el.innerHTML =
          '<div class="rem-checkbox' + (task.completed ? " checked" : "") + '" data-id="' + task.id + '">' + (task.completed ? "✓" : "") + '</div>' +
          '<div class="rem-task-body"><div class="rem-task-title' + (task.completed ? " completed" : "") + priClass + '">' + task.title + '</div>' +
          '<div class="rem-task-meta">' +
            (task.dueDate ? "<span>📅 " + task.dueDate + "</span>" : "") +
            (task.priority !== "none" ? "<span>⚡ " + task.priority + "</span>" : "") +
          '</div></div>' +
          '<span class="rem-task-flag" data-flag="' + task.id + '">' + (task.flagged ? "🚩" : "⚐") + "</span>";

        el.querySelector(".rem-checkbox").addEventListener("click", function () {
          task.completed = !task.completed;
          task.completedAt = task.completed ? Date.now() : null;
          renderTasks();
        });
        el.querySelector(".rem-task-flag").addEventListener("click", function () {
          task.flagged = !task.flagged;
          renderTasks();
        });
        remTaskList.appendChild(el);
      });
    }

    document.querySelectorAll("[data-rem-view]").forEach(function (btn) {
      btn.addEventListener("click", function () {
        document.querySelectorAll("[data-rem-view]").forEach(function (b) { b.classList.remove("rem-smart-active"); });
        btn.classList.add("rem-smart-active");
        currentView = btn.dataset.remView;
        renderTasks();
      });
    });

    remSort.addEventListener("change", renderTasks);

    remAddTaskBtn.addEventListener("click", function () {
      var title = prompt("Reminder title:");
      if (!title) return;
      tasks.push({ id: nextTaskId++, title: title, notes: "", dueDate: null, completed: false, completedAt: null, flagged: false, priority: "none", listId: 1 });
      renderTasks();
    });

    renderTasks();
  })();

  /* ══════════════════════════════════════════════════════
     20. CALENDAR EVENT MANAGER (Issue #34)
     ══════════════════════════════════════════════════════ */
  (function () {
    var calGrid = document.getElementById("cal-grid-full");
    var calLabel = document.getElementById("cal-month-label");
    var calPrev = document.getElementById("cal-prev");
    var calNext = document.getElementById("cal-next");
    var calGoToday = document.getElementById("cal-go-today");
    var calAddEvent = document.getElementById("cal-add-event");
    var calDayTitle = document.getElementById("cal-day-title");
    var calDayEvents = document.getElementById("cal-day-events");
    if (!calGrid) return;

    var calYear = 2026, calMonth = 2; // March (0-indexed)
    var calEvents = [
      { id: 1, title: "Team Standup", date: "2026-03-21", startTime: "09:00", endTime: "09:30", color: "#007aff" },
      { id: 2, title: "Design Review", date: "2026-03-21", startTime: "14:00", endTime: "15:00", color: "#34c759" },
      { id: 3, title: "Lunch with Sarah", date: "2026-03-22", startTime: "12:30", endTime: "13:30", color: "#ff9500" },
      { id: 4, title: "Sprint Planning", date: "2026-03-24", startTime: "10:00", endTime: "11:30", color: "#af52de" },
      { id: 5, title: "AuroraOS Beta Release", date: "2026-03-28", allDay: true, color: "#ff3b30" },
    ];
    var calNextId = 6;
    var selectedDate = null;
    var MONTH_NAMES = ["January", "February", "March", "April", "May", "June",
      "July", "August", "September", "October", "November", "December"];

    function renderCalendar() {
      calGrid.innerHTML = "";
      calLabel.textContent = MONTH_NAMES[calMonth] + " " + calYear;
      var dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
      dayNames.forEach(function (d) {
        var h = document.createElement("div");
        h.className = "cal-grid-header";
        h.textContent = d;
        calGrid.appendChild(h);
      });
      var firstDay = new Date(calYear, calMonth, 1).getDay();
      var daysInMonth = new Date(calYear, calMonth + 1, 0).getDate();
      var prevDays = new Date(calYear, calMonth, 0).getDate();
      var today = new Date();
      var todayStr = today.getFullYear() + "-" + String(today.getMonth() + 1).padStart(2, "0") + "-" + String(today.getDate()).padStart(2, "0");
      var prefix = calYear + "-" + String(calMonth + 1).padStart(2, "0") + "-";

      // Fill previous month
      for (var i = firstDay - 1; i >= 0; i--) {
        var cell = document.createElement("div");
        cell.className = "cal-grid-cell other-month";
        cell.innerHTML = '<div class="cal-cell-num">' + (prevDays - i) + "</div>";
        calGrid.appendChild(cell);
      }
      // Fill current month
      for (var d = 1; d <= daysInMonth; d++) {
        var dateStr = prefix + String(d).padStart(2, "0");
        var cell = document.createElement("div");
        var cls = "cal-grid-cell";
        if (dateStr === todayStr) cls += " today";
        if (dateStr === selectedDate) cls += " selected";
        cell.className = cls;
        var hasEvents = calEvents.some(function (e) { return e.date === dateStr; });
        cell.innerHTML = '<div class="cal-cell-num">' + d + "</div>" + (hasEvents ? '<span class="cal-cell-dot"></span>' : "");
        cell.dataset.date = dateStr;
        cell.addEventListener("click", function () {
          selectedDate = this.dataset.date;
          renderCalendar();
          renderDayDetail(this.dataset.date);
        });
        calGrid.appendChild(cell);
      }
      // Fill next month
      var total = firstDay + daysInMonth;
      var remaining = (7 - (total % 7)) % 7;
      for (var i = 1; i <= remaining; i++) {
        var cell = document.createElement("div");
        cell.className = "cal-grid-cell other-month";
        cell.innerHTML = '<div class="cal-cell-num">' + i + "</div>";
        calGrid.appendChild(cell);
      }
    }

    function renderDayDetail(date) {
      var parts = date.split("-");
      var dayDate = new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, parseInt(parts[2]));
      var dayNames = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
      calDayTitle.textContent = dayNames[dayDate.getDay()] + ", " + MONTH_NAMES[parseInt(parts[1]) - 1] + " " + parseInt(parts[2]);
      calDayEvents.innerHTML = "";
      var evts = calEvents.filter(function (e) { return e.date === date; });
      if (evts.length === 0) {
        calDayEvents.innerHTML = '<div class="cal-event-empty">No events</div>';
        return;
      }
      evts.forEach(function (ev) {
        var card = document.createElement("div");
        card.className = "cal-event-card";
        card.style.borderColor = ev.color || "var(--blue)";
        card.innerHTML = '<div class="cal-event-title">' + ev.title + "</div>" +
          (ev.startTime ? '<div class="cal-event-time">' + ev.startTime + " - " + ev.endTime + "</div>" : '<div class="cal-event-time">All day</div>');
        calDayEvents.appendChild(card);
      });
    }

    calPrev.addEventListener("click", function () {
      calMonth--;
      if (calMonth < 0) { calMonth = 11; calYear--; }
      renderCalendar();
    });
    calNext.addEventListener("click", function () {
      calMonth++;
      if (calMonth > 11) { calMonth = 0; calYear++; }
      renderCalendar();
    });
    calGoToday.addEventListener("click", function () {
      var now = new Date();
      calYear = now.getFullYear();
      calMonth = now.getMonth();
      selectedDate = calYear + "-" + String(calMonth + 1).padStart(2, "0") + "-" + String(now.getDate()).padStart(2, "0");
      renderCalendar();
      renderDayDetail(selectedDate);
    });
    calAddEvent.addEventListener("click", function () {
      if (!selectedDate) { alert("Select a date first"); return; }
      var title = prompt("Event title:");
      if (!title) return;
      var time = prompt("Start time (HH:MM) or leave empty for all-day:") || "";
      var ev = { id: calNextId++, title: title, date: selectedDate, color: "#007aff" };
      if (time) {
        ev.startTime = time;
        ev.endTime = prompt("End time (HH:MM):") || time;
      } else {
        ev.allDay = true;
      }
      calEvents.push(ev);
      renderCalendar();
      renderDayDetail(selectedDate);
    });

    renderCalendar();
  })();

  /* ══════════════════════════════════════════════════════
     21. STARTUP ITEMS MANAGER (Issue #57)
     ══════════════════════════════════════════════════════ */
  (function () {
    var startupList = document.getElementById("startup-list");
    var startupAddBtn = document.getElementById("startup-add-btn");
    var startupRemoveBtn = document.getElementById("startup-remove-btn");
    if (!startupList) return;

    var items = [
      { id: 1, name: "Safari", type: "app", enabled: true, hidden: false },
      { id: 2, name: "Mail", type: "app", enabled: true, hidden: false },
      { id: 3, name: "Messages", type: "app", enabled: false, hidden: false },
      { id: 4, name: "Cloud Sync", type: "background", enabled: true, hidden: true },
      { id: 5, name: "Spotlight Indexer", type: "agent", enabled: true, hidden: true },
    ];
    var nextId = 6;
    var selectedId = null;

    function renderStartup() {
      startupList.innerHTML = "";
      items.forEach(function (item) {
        var el = document.createElement("div");
        el.className = "startup-item" + (item.id === selectedId ? " selected" : "");
        var icon = item.type === "app" ? "🚀" : item.type === "background" ? "⚙️" : "🔧";
        el.innerHTML =
          '<span class="startup-item-icon">' + icon + "</span>" +
          '<div class="startup-item-info"><div class="startup-item-name">' + item.name + "</div>" +
          '<div class="startup-item-type">' + item.type + (item.hidden ? " (hidden)" : "") + "</div></div>" +
          '<div class="startup-item-toggle' + (item.enabled ? " active" : "") + '" data-id="' + item.id + '"></div>';
        el.addEventListener("click", function (e) {
          if (e.target.classList.contains("startup-item-toggle")) {
            item.enabled = !item.enabled;
            renderStartup();
            return;
          }
          selectedId = item.id;
          renderStartup();
        });
        startupList.appendChild(el);
      });
    }

    startupAddBtn.addEventListener("click", function () {
      var name = prompt("App name:");
      if (!name) return;
      items.push({ id: nextId++, name: name, type: "app", enabled: true, hidden: false });
      renderStartup();
    });

    startupRemoveBtn.addEventListener("click", function () {
      if (selectedId) {
        items = items.filter(function (i) { return i.id !== selectedId; });
        selectedId = null;
        renderStartup();
      }
    });

    renderStartup();
  })();

  /* ══════════════════════════════════════════════════════
     22. i18n LANGUAGE INTEGRATION (Issue #49)
     ══════════════════════════════════════════════════════ */
  (function () {
    // Translations
    var translations = {
      en: { "menu.file": "File", "menu.edit": "Edit", "menu.view": "View", "menu.go": "Go", "menu.window": "Window", "menu.help": "Help" },
      pt: { "menu.file": "Ficheiro", "menu.edit": "Editar", "menu.view": "Visualização", "menu.go": "Ir", "menu.window": "Janela", "menu.help": "Ajuda" },
      es: { "menu.file": "Archivo", "menu.edit": "Editar", "menu.view": "Vista", "menu.go": "Ir", "menu.window": "Ventana", "menu.help": "Ayuda" },
      fr: { "menu.file": "Fichier", "menu.edit": "Édition", "menu.view": "Présentation", "menu.go": "Aller", "menu.window": "Fenêtre", "menu.help": "Aide" },
      de: { "menu.file": "Ablage", "menu.edit": "Bearbeiten", "menu.view": "Darstellung", "menu.go": "Gehe zu", "menu.window": "Fenster", "menu.help": "Hilfe" },
    };
    var localeNames = { en: "English", pt: "Português", es: "Español", fr: "Français", de: "Deutsch" };
    var currentLocale = "en";

    // Create language selector element
    var langSelector = document.createElement("div");
    langSelector.className = "lang-selector";
    langSelector.id = "lang-selector";

    Object.keys(localeNames).forEach(function (code) {
      var opt = document.createElement("div");
      opt.className = "lang-option" + (code === currentLocale ? " active" : "");
      opt.textContent = localeNames[code];
      opt.dataset.locale = code;
      opt.addEventListener("click", function () {
        currentLocale = code;
        applyLocale(code);
        langSelector.classList.remove("visible");
        document.querySelectorAll(".lang-option").forEach(function (o) { o.classList.remove("active"); });
        opt.classList.add("active");
      });
      langSelector.appendChild(opt);
    });
    document.body.appendChild(langSelector);

    function applyLocale(locale) {
      var strings = translations[locale] || translations.en;
      var menuItems = document.querySelectorAll(".menu-left .menu-item:not(.apple-logo):not(.bold)");
      var menuKeys = ["menu.file", "menu.edit", "menu.view", "menu.go", "menu.window", "menu.help"];
      menuItems.forEach(function (item, i) {
        if (menuKeys[i] && strings[menuKeys[i]]) {
          item.textContent = strings[menuKeys[i]];
        }
      });
    }

    // Toggle language selector: Ctrl+Shift+L
    document.addEventListener("keydown", function (e) {
      if (e.ctrlKey && e.shiftKey && e.key === "L") {
        e.preventDefault();
        langSelector.classList.toggle("visible");
      }
    });

    // Also wire into Launchpad entry
    window._auroraI18n = { setLocale: applyLocale, getLocale: function () { return currentLocale; } };
  })();

  /* ── Open new windows from Launchpad / Dock ──────── */
  (function () {
    var appWindowMap = {
      "Keychain Access": ".window-keychain",
      "Reminders": ".window-reminders",
      "Calendar": ".window-calendar-full",
      "Login Items": ".window-startup",
    };

    // Wire dock icon clicks to open windows
    document.querySelectorAll(".dock-icon[data-app]").forEach(function (icon) {
      icon.addEventListener("click", function () {
        var appName = icon.dataset.app;
        // Reminders dock icon
        if (appName === "Reminders") openAppWindow(".window-reminders");
        if (appName === "Calendar") openAppWindow(".window-calendar-full");
      });
    });

    function openAppWindow(selector) {
      var win = document.querySelector(selector);
      if (win) {
        win.style.display = "";
        win.style.opacity = "1";
        win.style.transform = "";
        focusWindow(win);
      }
    }

    // Expose for launchpad
    window._auroraOpenApp = openAppWindow;
  })();

})();
