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

})();
