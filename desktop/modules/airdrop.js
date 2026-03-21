/**
 * AirDrop Manager (Issue #57)
 * Manage AirDrop file transfers between nearby Apple devices.
 */

var VALID_VISIBILITIES = ['no-one', 'contacts-only', 'everyone'];
var VALID_FILE_TYPES = ['image', 'video', 'document', 'contact', 'link', 'other'];

class AirDropManager {
  constructor(options) {
    options = options || {};
    this.enabled = options.enabled || false;
    this.visibility = options.visibility || 'contacts-only';
    this.nearbyDevices = new Map();
    this.transfers = new Map();
    this.transferHistory = [];
    this.maxConcurrent = 3;
    this.maxFileSize = 5 * 1024 * 1024 * 1024;
    this.listeners = { request: [], progress: [], complete: [] };
    this._nextTransferId = 1;
  }

  enable() {
    this.enabled = true;
  }

  disable() {
    this.enabled = false;
  }

  setVisibility(value) {
    if (VALID_VISIBILITIES.indexOf(value) === -1) {
      throw new Error('Invalid visibility: ' + value);
    }
    this.visibility = value;
  }

  addNearbyDevice(device) {
    this.nearbyDevices.set(device.id, {
      id: device.id,
      name: device.name,
      type: device.type,
      distance: device.distance,
    });
  }

  removeNearbyDevice(deviceId) {
    this.nearbyDevices.delete(deviceId);
  }

  discoverDevices() {
    if (!this.enabled) {
      return [];
    }
    var devices = [];
    this.nearbyDevices.forEach(function (device) {
      devices.push(device);
    });
    return devices;
  }

  sendFile(deviceId, file) {
    if (!this.enabled) {
      return { success: false, error: 'AirDrop is disabled' };
    }
    if (!this.nearbyDevices.has(deviceId)) {
      return { success: false, error: 'Device not found' };
    }
    if (VALID_FILE_TYPES.indexOf(file.type) === -1) {
      return { success: false, error: 'Unsupported file type' };
    }
    if (file.size > this.maxFileSize) {
      return { success: false, error: 'File exceeds maximum size of 5GB' };
    }

    var sendingCount = 0;
    this.transfers.forEach(function (t) {
      if (t.state === 'sending') {
        sendingCount++;
      }
    });
    if (sendingCount >= this.maxConcurrent) {
      return { success: false, error: 'Maximum concurrent transfers reached' };
    }

    var id = this._nextTransferId++;
    var transfer = {
      id: id,
      deviceId: deviceId,
      file: file,
      state: 'pending',
      progress: 0,
      direction: 'outgoing',
      timestamp: Date.now(),
    };
    this.transfers.set(id, transfer);
    return { success: true, transferId: id };
  }

  sendFiles(deviceId, files) {
    var transferIds = [];
    for (var i = 0; i < files.length; i++) {
      var result = this.sendFile(deviceId, files[i]);
      if (!result.success) {
        return result;
      }
      transferIds.push(result.transferId);
    }
    return { success: true, transferIds: transferIds };
  }

  createIncomingTransfer(file, fromDeviceId) {
    var id = this._nextTransferId++;
    var transfer = {
      id: id,
      deviceId: fromDeviceId,
      file: file,
      state: 'pending',
      progress: 0,
      direction: 'incoming',
      timestamp: Date.now(),
    };
    this.transfers.set(id, transfer);

    for (var i = 0; i < this.listeners.request.length; i++) {
      this.listeners.request[i](transfer);
    }

    return id;
  }

  acceptTransfer(transferId) {
    var transfer = this.transfers.get(transferId);
    if (transfer) {
      transfer.state = 'sending';
    }
  }

  rejectTransfer(transferId) {
    var transfer = this.transfers.get(transferId);
    if (transfer) {
      transfer.state = 'rejected';
    }
  }

  updateTransferState(transferId, state) {
    var transfer = this.transfers.get(transferId);
    if (!transfer) {
      return;
    }
    transfer.state = state;

    if (state === 'completed' || state === 'failed' || state === 'rejected') {
      this.transferHistory.push({
        id: transfer.id,
        deviceId: transfer.deviceId,
        file: transfer.file,
        state: transfer.state,
        direction: transfer.direction,
        timestamp: transfer.timestamp,
        completedAt: Date.now(),
      });
    }

    if (state === 'completed') {
      for (var i = 0; i < this.listeners.complete.length; i++) {
        this.listeners.complete[i](transfer);
      }
    }
  }

  updateTransferProgress(transferId, progress) {
    var transfer = this.transfers.get(transferId);
    if (!transfer) {
      return;
    }
    if (progress < 0) {
      progress = 0;
    }
    if (progress > 100) {
      progress = 100;
    }
    transfer.progress = progress;

    for (var i = 0; i < this.listeners.progress.length; i++) {
      this.listeners.progress[i](transferId, progress);
    }
  }

  cancelTransfer(transferId) {
    var transfer = this.transfers.get(transferId);
    if (!transfer) {
      return false;
    }
    if (transfer.state === 'completed' || transfer.state === 'rejected') {
      return false;
    }
    transfer.state = 'failed';
    this.transferHistory.push({
      id: transfer.id,
      deviceId: transfer.deviceId,
      file: transfer.file,
      state: 'failed',
      direction: transfer.direction,
      timestamp: transfer.timestamp,
      completedAt: Date.now(),
    });
    return true;
  }

  getActiveTransfers() {
    var active = [];
    this.transfers.forEach(function (transfer) {
      if (transfer.state === 'pending' || transfer.state === 'sending') {
        active.push(transfer);
      }
    });
    return active;
  }

  getTransferHistory() {
    return this.transferHistory.slice();
  }

  clearTransferHistory() {
    this.transferHistory = [];
  }

  retryTransfer(transferId) {
    var transfer = this.transfers.get(transferId);
    if (!transfer) {
      return { success: false, error: 'Transfer not found' };
    }
    if (transfer.state !== 'failed') {
      return { success: false, error: 'Transfer is not in failed state' };
    }
    return this.sendFile(transfer.deviceId, transfer.file);
  }

  onTransferRequest(handler) {
    this.listeners.request.push(handler);
  }

  onTransferProgress(handler) {
    this.listeners.progress.push(handler);
  }

  onTransferComplete(handler) {
    this.listeners.complete.push(handler);
  }
}

module.exports = { AirDropManager };
