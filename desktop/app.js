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

})();
