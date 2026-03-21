/**
 * Window Snapping System
 * Snap windows to screen edges and corners with preview support.
 */

class WindowSnapping {
  constructor(options) {
    options = options || {};
    this.screenWidth = options.screenWidth || 1440;
    this.screenHeight = options.screenHeight || 900;
    this.menuBarHeight = options.menuBarHeight || 25;
    this.dockHeight = options.dockHeight || 70;
    this.edgeThreshold = options.edgeThreshold || 20;
    this.windows = new Map();
    this.snapHistory = new Map();
  }

  _usableTop() {
    return this.menuBarHeight;
  }

  _usableHeight() {
    return this.screenHeight - this.menuBarHeight - this.dockHeight;
  }

  _halfWidth() {
    return Math.floor(this.screenWidth / 2);
  }

  _halfHeight() {
    return Math.floor(this._usableHeight() / 2);
  }

  _zoneRect(zone) {
    var top = this._usableTop();
    var uh = this._usableHeight();
    var hw = this._halfWidth();
    var hh = this._halfHeight();

    switch (zone) {
      case 'left':
        return { x: 0, y: top, width: hw, height: uh };
      case 'right':
        return { x: hw, y: top, width: this.screenWidth - hw, height: uh };
      case 'top':
        return { x: 0, y: top, width: this.screenWidth, height: hh };
      case 'bottom':
        return { x: 0, y: top + hh, width: this.screenWidth, height: uh - hh };
      case 'topLeft':
        return { x: 0, y: top, width: hw, height: hh };
      case 'topRight':
        return { x: hw, y: top, width: this.screenWidth - hw, height: hh };
      case 'bottomLeft':
        return { x: 0, y: top + hh, width: hw, height: uh - hh };
      case 'bottomRight':
        return { x: hw, y: top + hh, width: this.screenWidth - hw, height: uh - hh };
      case 'maximized':
        return { x: 0, y: top, width: this.screenWidth, height: uh };
      default:
        return null;
    }
  }

  registerWindow(windowId, rect) {
    this.windows.set(windowId, {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      snapped: false,
      snapZone: null,
    });
  }

  getWindowState(windowId) {
    var state = this.windows.get(windowId);
    if (!state) return null;
    return {
      x: state.x,
      y: state.y,
      width: state.width,
      height: state.height,
      snapped: state.snapped,
      snapZone: state.snapZone,
    };
  }

  _applySnap(windowId, zone) {
    var state = this.windows.get(windowId);
    if (!state) return;

    if (!state.snapped) {
      this.snapHistory.set(windowId, {
        x: state.x,
        y: state.y,
        width: state.width,
        height: state.height,
      });
    }

    this._displaceExisting(zone, windowId);

    var rect = this._zoneRect(zone);
    state.x = rect.x;
    state.y = rect.y;
    state.width = rect.width;
    state.height = rect.height;
    state.snapped = true;
    state.snapZone = zone;
  }

  _displaceExisting(zone, excludeId) {
    this.windows.forEach(function (state, id) {
      if (id !== excludeId && state.snapped && state.snapZone === zone) {
        var original = this.snapHistory.get(id);
        if (original) {
          state.x = original.x;
          state.y = original.y;
          state.width = original.width;
          state.height = original.height;
        }
        state.snapped = false;
        state.snapZone = null;
      }
    }.bind(this));
  }

  snapLeft(windowId) {
    this._applySnap(windowId, 'left');
  }

  snapRight(windowId) {
    this._applySnap(windowId, 'right');
  }

  snapTop(windowId) {
    this._applySnap(windowId, 'top');
  }

  snapBottom(windowId) {
    this._applySnap(windowId, 'bottom');
  }

  snapTopLeft(windowId) {
    this._applySnap(windowId, 'topLeft');
  }

  snapTopRight(windowId) {
    this._applySnap(windowId, 'topRight');
  }

  snapBottomLeft(windowId) {
    this._applySnap(windowId, 'bottomLeft');
  }

  snapBottomRight(windowId) {
    this._applySnap(windowId, 'bottomRight');
  }

  maximize(windowId) {
    this._applySnap(windowId, 'maximized');
  }

  restore(windowId) {
    var state = this.windows.get(windowId);
    if (!state || !state.snapped) return;

    var original = this.snapHistory.get(windowId);
    if (original) {
      state.x = original.x;
      state.y = original.y;
      state.width = original.width;
      state.height = original.height;
    }
    state.snapped = false;
    state.snapZone = null;
  }

  unsnap(windowId, newX, newY) {
    var state = this.windows.get(windowId);
    if (!state) return;

    var original = this.snapHistory.get(windowId);
    if (original) {
      state.width = original.width;
      state.height = original.height;
    }
    state.x = newX;
    state.y = newY;
    state.snapped = false;
    state.snapZone = null;
  }

  getSnapZone(x, y) {
    var t = this.edgeThreshold;
    var nearLeft = x < t;
    var nearRight = x >= this.screenWidth - t;
    var nearTop = y < t;
    var nearBottom = y >= this.screenHeight - t;

    if (nearLeft && nearTop) return 'topLeft';
    if (nearRight && nearTop) return 'topRight';
    if (nearLeft && nearBottom) return 'bottomLeft';
    if (nearRight && nearBottom) return 'bottomRight';
    if (nearLeft) return 'left';
    if (nearRight) return 'right';
    if (nearTop) return 'top';
    if (nearBottom) return 'bottom';
    return null;
  }

  getPreviewRect(zone) {
    return this._zoneRect(zone);
  }

  getSnappedWindows() {
    var result = [];
    this.windows.forEach(function (state, id) {
      if (state.snapped) {
        result.push(id);
      }
    });
    return result;
  }

  cascadeUnsnapped() {
    var offset = 0;
    var baseX = 50;
    var baseY = this._usableTop() + 50;
    this.windows.forEach(function (state) {
      if (!state.snapped) {
        state.x = baseX + offset;
        state.y = baseY + offset;
        offset += 30;
      }
    });
  }
}

module.exports = { WindowSnapping };
