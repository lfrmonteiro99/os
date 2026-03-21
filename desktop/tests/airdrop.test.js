/**
 * TDD Tests for AirDrop Manager (Issue #57)
 * RED phase: write tests before implementation
 */
const { AirDropManager } = require('../modules/airdrop');

describe('AirDropManager', () => {
  let airdrop;

  beforeEach(() => {
    airdrop = new AirDropManager();
  });

  describe('constructor', () => {
    test('defaults enabled to false', () => {
      expect(airdrop.enabled).toBe(false);
    });

    test('defaults visibility to contacts-only', () => {
      expect(airdrop.visibility).toBe('contacts-only');
    });

    test('accepts custom options', () => {
      const custom = new AirDropManager({ enabled: true, visibility: 'everyone' });
      expect(custom.enabled).toBe(true);
      expect(custom.visibility).toBe('everyone');
    });

    test('starts with empty nearby devices', () => {
      expect(airdrop.nearbyDevices.size).toBe(0);
    });

    test('starts with empty transfers', () => {
      expect(airdrop.transfers.size).toBe(0);
    });

    test('starts with empty transfer history', () => {
      expect(airdrop.transferHistory).toEqual([]);
    });

    test('sets max concurrent transfers to 3', () => {
      expect(airdrop.maxConcurrent).toBe(3);
    });

    test('sets max file size to 5GB', () => {
      expect(airdrop.maxFileSize).toBe(5 * 1024 * 1024 * 1024);
    });
  });

  describe('enable() / disable()', () => {
    test('enable sets enabled to true', () => {
      airdrop.enable();
      expect(airdrop.enabled).toBe(true);
    });

    test('disable sets enabled to false', () => {
      airdrop.enable();
      airdrop.disable();
      expect(airdrop.enabled).toBe(false);
    });
  });

  describe('setVisibility()', () => {
    test('sets visibility to no-one', () => {
      airdrop.setVisibility('no-one');
      expect(airdrop.visibility).toBe('no-one');
    });

    test('sets visibility to contacts-only', () => {
      airdrop.setVisibility('contacts-only');
      expect(airdrop.visibility).toBe('contacts-only');
    });

    test('sets visibility to everyone', () => {
      airdrop.setVisibility('everyone');
      expect(airdrop.visibility).toBe('everyone');
    });

    test('throws on invalid visibility value', () => {
      expect(() => airdrop.setVisibility('invalid')).toThrow('Invalid visibility');
    });
  });

  describe('discoverDevices()', () => {
    test('returns array of nearby devices', () => {
      airdrop.enable();
      const devices = airdrop.discoverDevices();
      expect(Array.isArray(devices)).toBe(true);
    });

    test('each device has name, type, and distance', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'MacBook Pro', type: 'mac', distance: 2.5 });
      const devices = airdrop.discoverDevices();
      expect(devices[0]).toEqual(expect.objectContaining({
        id: 'dev1',
        name: 'MacBook Pro',
        type: 'mac',
        distance: 2.5,
      }));
    });

    test('returns empty array when disabled', () => {
      airdrop.addNearbyDevice({ id: 'dev1', name: 'MacBook Pro', type: 'mac', distance: 2.5 });
      const devices = airdrop.discoverDevices();
      expect(devices).toEqual([]);
    });
  });

  describe('sendFile()', () => {
    beforeEach(() => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'iPhone 15', type: 'iphone', distance: 1.0 });
    });

    test('initiates a transfer and returns transfer id', () => {
      const result = airdrop.sendFile('dev1', { name: 'photo.jpg', size: 1024, type: 'image' });
      expect(result.success).toBe(true);
      expect(result.transferId).toBeDefined();
    });

    test('transfer starts in pending state', () => {
      const result = airdrop.sendFile('dev1', { name: 'photo.jpg', size: 1024, type: 'image' });
      const transfer = airdrop.transfers.get(result.transferId);
      expect(transfer.state).toBe('pending');
    });

    test('fails when airdrop is disabled', () => {
      airdrop.disable();
      const result = airdrop.sendFile('dev1', { name: 'photo.jpg', size: 1024, type: 'image' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('AirDrop is disabled');
    });

    test('fails for unknown device', () => {
      const result = airdrop.sendFile('unknown', { name: 'photo.jpg', size: 1024, type: 'image' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Device not found');
    });
  });

  describe('transfer states', () => {
    let transferId;

    beforeEach(() => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'iPhone 15', type: 'iphone', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'doc.pdf', size: 2048, type: 'document' });
      transferId = result.transferId;
    });

    test('can transition from pending to sending', () => {
      airdrop.updateTransferState(transferId, 'sending');
      expect(airdrop.transfers.get(transferId).state).toBe('sending');
    });

    test('can transition from sending to completed', () => {
      airdrop.updateTransferState(transferId, 'sending');
      airdrop.updateTransferState(transferId, 'completed');
      expect(airdrop.transfers.get(transferId).state).toBe('completed');
    });

    test('can transition to failed', () => {
      airdrop.updateTransferState(transferId, 'sending');
      airdrop.updateTransferState(transferId, 'failed');
      expect(airdrop.transfers.get(transferId).state).toBe('failed');
    });

    test('can transition from pending to rejected', () => {
      airdrop.updateTransferState(transferId, 'rejected');
      expect(airdrop.transfers.get(transferId).state).toBe('rejected');
    });

    test('completed transfers are added to history', () => {
      airdrop.updateTransferState(transferId, 'sending');
      airdrop.updateTransferState(transferId, 'completed');
      expect(airdrop.transferHistory.length).toBe(1);
      expect(airdrop.transferHistory[0].state).toBe('completed');
    });
  });

  describe('accept/reject incoming transfers', () => {
    test('acceptTransfer sets state to sending', () => {
      const id = airdrop.createIncomingTransfer({ name: 'file.zip', size: 500, type: 'other' }, 'dev2');
      airdrop.acceptTransfer(id);
      expect(airdrop.transfers.get(id).state).toBe('sending');
    });

    test('rejectTransfer sets state to rejected', () => {
      const id = airdrop.createIncomingTransfer({ name: 'file.zip', size: 500, type: 'other' }, 'dev2');
      airdrop.rejectTransfer(id);
      expect(airdrop.transfers.get(id).state).toBe('rejected');
    });
  });

  describe('transfer progress', () => {
    test('tracks progress from 0 to 100', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'iPad', type: 'ipad', distance: 3.0 });
      const result = airdrop.sendFile('dev1', { name: 'video.mp4', size: 4096, type: 'video' });
      const transfer = airdrop.transfers.get(result.transferId);
      expect(transfer.progress).toBe(0);

      airdrop.updateTransferProgress(result.transferId, 50);
      expect(airdrop.transfers.get(result.transferId).progress).toBe(50);

      airdrop.updateTransferProgress(result.transferId, 100);
      expect(airdrop.transfers.get(result.transferId).progress).toBe(100);
    });

    test('clamps progress between 0 and 100', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'iPad', type: 'ipad', distance: 3.0 });
      const result = airdrop.sendFile('dev1', { name: 'video.mp4', size: 4096, type: 'video' });
      airdrop.updateTransferProgress(result.transferId, 150);
      expect(airdrop.transfers.get(result.transferId).progress).toBe(100);

      airdrop.updateTransferProgress(result.transferId, -10);
      expect(airdrop.transfers.get(result.transferId).progress).toBe(0);
    });
  });

  describe('transfer history', () => {
    test('getTransferHistory returns copy of history', () => {
      const history = airdrop.getTransferHistory();
      expect(history).toEqual([]);
      expect(history).not.toBe(airdrop.transferHistory);
    });

    test('clearTransferHistory empties the history', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'a.txt', size: 10, type: 'document' });
      airdrop.updateTransferState(result.transferId, 'sending');
      airdrop.updateTransferState(result.transferId, 'completed');
      expect(airdrop.getTransferHistory().length).toBe(1);
      airdrop.clearTransferHistory();
      expect(airdrop.getTransferHistory()).toEqual([]);
    });
  });

  describe('file type support', () => {
    test('supports image, video, document, contact, link, other', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const types = ['image', 'video', 'document', 'contact', 'link', 'other'];
      types.forEach(type => {
        const result = airdrop.sendFile('dev1', { name: 'file', size: 100, type: type });
        expect(result.success).toBe(true);
      });
    });

    test('rejects unsupported file type', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'file', size: 100, type: 'executable' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Unsupported file type');
    });
  });

  describe('file size validation', () => {
    test('rejects files exceeding 5GB', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const bigSize = 6 * 1024 * 1024 * 1024;
      const result = airdrop.sendFile('dev1', { name: 'huge.iso', size: bigSize, type: 'other' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('File exceeds maximum size of 5GB');
    });

    test('accepts files at exactly 5GB', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const exactSize = 5 * 1024 * 1024 * 1024;
      const result = airdrop.sendFile('dev1', { name: 'big.iso', size: exactSize, type: 'other' });
      expect(result.success).toBe(true);
    });
  });

  describe('batch transfer', () => {
    test('sendFiles sends multiple files and returns transfer ids', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const files = [
        { name: 'a.jpg', size: 100, type: 'image' },
        { name: 'b.jpg', size: 200, type: 'image' },
        { name: 'c.jpg', size: 300, type: 'image' },
      ];
      const result = airdrop.sendFiles('dev1', files);
      expect(result.success).toBe(true);
      expect(result.transferIds.length).toBe(3);
    });
  });

  describe('cancel transfer', () => {
    test('cancels a transfer in progress', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'file.zip', size: 1024, type: 'other' });
      airdrop.updateTransferState(result.transferId, 'sending');
      const cancelled = airdrop.cancelTransfer(result.transferId);
      expect(cancelled).toBe(true);
      expect(airdrop.transfers.get(result.transferId).state).toBe('failed');
    });

    test('cannot cancel a completed transfer', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'file.zip', size: 1024, type: 'other' });
      airdrop.updateTransferState(result.transferId, 'sending');
      airdrop.updateTransferState(result.transferId, 'completed');
      const cancelled = airdrop.cancelTransfer(result.transferId);
      expect(cancelled).toBe(false);
    });
  });

  describe('getActiveTransfers()', () => {
    test('returns transfers in pending or sending state', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      airdrop.sendFile('dev1', { name: 'a.jpg', size: 100, type: 'image' });
      const r2 = airdrop.sendFile('dev1', { name: 'b.jpg', size: 200, type: 'image' });
      airdrop.updateTransferState(r2.transferId, 'sending');
      airdrop.updateTransferState(r2.transferId, 'completed');
      const active = airdrop.getActiveTransfers();
      expect(active.length).toBe(1);
    });
  });

  describe('device info', () => {
    test('device has name, type, and distance', () => {
      airdrop.addNearbyDevice({ id: 'dev1', name: 'MacBook Air', type: 'mac', distance: 4.2 });
      const device = airdrop.nearbyDevices.get('dev1');
      expect(device.name).toBe('MacBook Air');
      expect(device.type).toBe('mac');
      expect(device.distance).toBe(4.2);
    });

    test('supports iphone, ipad, and mac device types', () => {
      airdrop.addNearbyDevice({ id: 'd1', name: 'iPhone', type: 'iphone', distance: 1 });
      airdrop.addNearbyDevice({ id: 'd2', name: 'iPad', type: 'ipad', distance: 2 });
      airdrop.addNearbyDevice({ id: 'd3', name: 'Mac', type: 'mac', distance: 3 });
      expect(airdrop.nearbyDevices.size).toBe(3);
    });
  });

  describe('callbacks', () => {
    test('onTransferRequest fires for incoming transfers', () => {
      const handler = jest.fn();
      airdrop.onTransferRequest(handler);
      airdrop.createIncomingTransfer({ name: 'pic.png', size: 500, type: 'image' }, 'dev2');
      expect(handler).toHaveBeenCalledWith(expect.objectContaining({
        file: expect.objectContaining({ name: 'pic.png' }),
      }));
    });

    test('onTransferProgress fires when progress updates', () => {
      const handler = jest.fn();
      airdrop.onTransferProgress(handler);
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1 });
      const result = airdrop.sendFile('dev1', { name: 'f.txt', size: 10, type: 'document' });
      airdrop.updateTransferProgress(result.transferId, 42);
      expect(handler).toHaveBeenCalledWith(result.transferId, 42);
    });

    test('onTransferComplete fires when transfer completes', () => {
      const handler = jest.fn();
      airdrop.onTransferComplete(handler);
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1 });
      const result = airdrop.sendFile('dev1', { name: 'f.txt', size: 10, type: 'document' });
      airdrop.updateTransferState(result.transferId, 'sending');
      airdrop.updateTransferState(result.transferId, 'completed');
      expect(handler).toHaveBeenCalledWith(expect.objectContaining({
        id: result.transferId,
        state: 'completed',
      }));
    });
  });

  describe('retry failed transfer', () => {
    test('retries a failed transfer and creates a new transfer', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'doc.pdf', size: 2048, type: 'document' });
      airdrop.updateTransferState(result.transferId, 'sending');
      airdrop.updateTransferState(result.transferId, 'failed');
      const retry = airdrop.retryTransfer(result.transferId);
      expect(retry.success).toBe(true);
      expect(retry.transferId).not.toBe(result.transferId);
      expect(airdrop.transfers.get(retry.transferId).state).toBe('pending');
    });

    test('cannot retry a non-failed transfer', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const result = airdrop.sendFile('dev1', { name: 'doc.pdf', size: 2048, type: 'document' });
      const retry = airdrop.retryTransfer(result.transferId);
      expect(retry.success).toBe(false);
      expect(retry.error).toBe('Transfer is not in failed state');
    });
  });

  describe('concurrent transfer limit', () => {
    test('rejects transfer when at max concurrent limit', () => {
      airdrop.enable();
      airdrop.addNearbyDevice({ id: 'dev1', name: 'Mac', type: 'mac', distance: 1.0 });
      const r1 = airdrop.sendFile('dev1', { name: 'a.jpg', size: 100, type: 'image' });
      airdrop.updateTransferState(r1.transferId, 'sending');
      const r2 = airdrop.sendFile('dev1', { name: 'b.jpg', size: 100, type: 'image' });
      airdrop.updateTransferState(r2.transferId, 'sending');
      const r3 = airdrop.sendFile('dev1', { name: 'c.jpg', size: 100, type: 'image' });
      airdrop.updateTransferState(r3.transferId, 'sending');
      const r4 = airdrop.sendFile('dev1', { name: 'd.jpg', size: 100, type: 'image' });
      expect(r4.success).toBe(false);
      expect(r4.error).toBe('Maximum concurrent transfers reached');
    });
  });
});
