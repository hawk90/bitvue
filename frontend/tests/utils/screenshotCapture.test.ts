/**
 * Screenshot Capture Utility Tests
 * Tests canvas capture and download functionality
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  captureCanvas,
  downloadDataUrl,
  captureFrame,
  copyCanvasToClipboard,
  getTimestampFilename,
} from '../screenshotCapture';

// Mock toDataURL and clipboard API
const mockToDataURL = vi.fn();
const mockToBlob = vi.fn();
const mockWrite = vi.fn();
const mockClick = vi.fn();
const mockRemoveChild = vi.fn();

describe('captureCanvas', () => {
  let mockCanvas: HTMLCanvasElement;

  beforeEach(() => {
    mockCanvas = {
      toDataURL: mockToDataURL,
    } as unknown as HTMLCanvasElement;
    mockToDataURL.mockReturnValue('data:image/png;base64,mockdata');
  });

  it('should capture canvas as PNG data URL', () => {
    const result = captureCanvas(mockCanvas);

    expect(mockToDataURL).toHaveBeenCalledWith('image/png');
    expect(result).toBe('data:image/png;base64,mockdata');
  });

  it('should return data URL string', () => {
    const result = captureCanvas(mockCanvas);

    expect(typeof result).toBe('string');
    expect(result).toMatch(/^data:image\/png;base64,/);
  });
});

describe('downloadDataUrl', () => {
  let mockLink: HTMLAnchorElement;

  beforeEach(() => {
    mockLink = {
      href: '',
      download: '',
      click: mockClick,
    } as unknown as HTMLAnchorElement;

    vi.spyOn(document, 'createElement').mockReturnValue(mockLink);
    vi.spyOn(document.body, 'appendChild').mockReturnValue(mockLink);
    mockRemoveChild.mockClear();
    vi.spyOn(document.body, 'removeChild').mockImplementation(mockRemoveChild);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should create anchor element', () => {
    downloadDataUrl('data:image/png;base64,test', 'test.png');

    expect(document.createElement).toHaveBeenCalledWith('a');
  });

  it('should set href and download attributes', () => {
    downloadDataUrl('data:image/png;base64,test', 'test.png');

    expect(mockLink.href).toBe('data:image/png;base64,test');
    expect(mockLink.download).toBe('test.png');
  });

  it('should click the link', () => {
    downloadDataUrl('data:image/png;base64,test', 'test.png');

    expect(mockClick).toHaveBeenCalled();
  });

  it('should append and remove link from body', () => {
    downloadDataUrl('data:image/png;base64,test', 'test.png');

    expect(document.body.appendChild).toHaveBeenCalledWith(mockLink);
    expect(mockRemoveChild).toHaveBeenCalledWith(mockLink);
  });
});

describe('captureFrame', () => {
  let mockCanvas: HTMLCanvasElement;

  beforeEach(() => {
    mockCanvas = {
      toDataURL: mockToDataURL,
    } as unknown as HTMLCanvasElement;
    mockToDataURL.mockReturnValue('data:image/png;base64,mockdata');

    vi.spyOn(document, 'createElement').mockReturnValue({
      click: mockClick,
    } as unknown as HTMLAnchorElement);
    vi.spyOn(document.body, 'appendChild').mockReturnValue({} as HTMLElement);
    vi.spyOn(document.body, 'removeChild').mockImplementation(mockRemoveChild);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should capture canvas and download', () => {
    captureFrame(mockCanvas, 5);

    expect(mockToDataURL).toHaveBeenCalledWith('image/png');
    expect(mockClick).toHaveBeenCalled();
  });

  it('should include frame index in filename', () => {
    captureFrame(mockCanvas, 10);

    const createElementCalls = (document.createElement as vi.Mock).mock.calls;
    const link = createElementCalls[createElementCalls.length - 1];

    expect(link[0]).toBe('a');
  });

  it('should include timestamp in filename', () => {
    captureFrame(mockCanvas, 1);

    expect(mockToDataURL).toHaveBeenCalled();
  });
});

describe('copyCanvasToClipboard', () => {
  let mockCanvas: HTMLCanvasElement;
  let mockBlob: Blob;

  beforeEach(() => {
    mockBlob = new Blob(['test'], { type: 'image/png' });

    mockCanvas = {
      toBlob: mockToBlob,
    } as unknown as HTMLCanvasElement;

    Object.defineProperty(navigator, 'clipboard', {
      value: {
        write: mockWrite,
      },
      writable: true,
      configurable: true,
    });
  });

  it('should copy canvas to clipboard successfully', async () => {
    mockToBlob.mockImplementation((callback: (blob: Blob) => void) => {
      callback(mockBlob);
    });
    mockWrite.mockResolvedValue(undefined);

    const result = await copyCanvasToClipboard(mockCanvas);

    expect(mockToBlob).toHaveBeenCalledWith(expect.any(Function), 'image/png');
    expect(mockWrite).toHaveBeenCalledWith([
      expect.objectContaining({
        'image/png': mockBlob,
      }),
    ]);
    expect(result).toBe(true);
  });

  it('should return false on error', async () => {
    mockToBlob.mockImplementation((callback: (blob: Blob) => void) => {
      callback(mockBlob);
    });
    mockWrite.mockRejectedValue(new Error('Clipboard error'));

    const result = await copyCanvasToClipboard(mockCanvas);

    expect(result).toBe(false);
  });

  it('should handle toBlob errors', async () => {
    mockToBlob.mockImplementation(() => {
      throw new Error('Blob error');
    });

    const result = await copyCanvasToClipboard(mockCanvas);

    expect(result).toBe(false);
  });
});

describe('getTimestampFilename', () => {
  it('should generate filename with prefix and frame index', () => {
    const filename = getTimestampFilename('bitvue', 5);

    expect(filename).toContain('bitvue-frame-5-');
    expect(filename).toContain('.png');
  });

  it('should include date in filename', () => {
    const filename = getTimestampFilename('test', 0);

    expect(filename).toMatch(/\d{4}-\d{2}-\d{2}/); // YYYY-MM-DD format
  });

  it('should include time in filename', () => {
    const filename = getTimestampFilename('test', 0);

    expect(filename).toMatch(/\d{2}-\d{2}-\d{2}/); // HH-MM-SS format
  });

  it('should separate date and time with underscore', () => {
    const filename = getTimestampFilename('test', 0);

    expect(filename).toMatch(/_\d{2}-\d{2}-\d{2}\.png$/);
  });

  it('should handle different prefixes', () => {
    const filename1 = getTimestampFilename('screenshot', 1);
    const filename2 = getTimestampFilename('capture', 2);

    expect(filename1).toContain('screenshot-frame-1-');
    expect(filename2).toContain('capture-frame-2-');
  });

  it('should handle large frame indices', () => {
    const filename = getTimestampFilename('test', 99999);

    expect(filename).toContain('frame-99999-');
  });
});

describe('screenshotCapture edge cases', () => {
  it('should handle frame index 0', () => {
    const filename = getTimestampFilename('test', 0);

    expect(filename).toContain('frame-0-');
  });

  it('should handle special characters in prefix', () => {
    const filename = getTimestampFilename('test-file', 1);

    expect(filename).toContain('test-file-frame-1-');
  });
});
