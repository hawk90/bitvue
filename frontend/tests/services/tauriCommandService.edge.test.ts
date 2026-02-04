/**
 * Tauri Command Service Edge Case Tests
 *
 * Tests boundary conditions, abnormal inputs, and error handling
 * for the Tauri command service.
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { TauriCommandService, TauriCommandError, invokeCommand, safeInvokeCommand } from '../tauriCommandService';

// Mock the Tauri invoke function
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

describe('TauriCommandService edge cases', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    // Clear latency stats
    TauriCommandService.clearLatencyStats();
  });

  afterEach(() => {
    mockInvoke.mockReset();
  });

  describe('abnormal command names', () => {
    it('should handle empty command name', async () => {
      mockInvoke.mockRejectedValue(new Error('Command not found'));

      await expect(TauriCommandService.invoke('')).rejects.toThrow(TauriCommandError);
    });

    it('should handle very long command names', async () => {
      const longCommand = 'a'.repeat(10000);
      mockInvoke.mockRejectedValue(new Error('Command not found'));

      await expect(TauriCommandService.invoke(longCommand)).rejects.toThrow(TauriCommandError);
    });

    it('should handle command names with special characters', async () => {
      const specialCommands = [
        'cmd-with-dashes',
        'cmd_with_underscores',
        'cmd.with.dots',
        'cmd:with:colons',
        'cmd/with/slashes',
      ];

      for (const cmd of specialCommands) {
        mockInvoke.mockRejectedValue(new Error('Command not found'));
        await expect(TauriCommandService.invoke(cmd)).rejects.toThrow();
      }
    });
  });

  describe('abnormal arguments', () => {
    it('should handle null arguments', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      // @ts-expect-error - Testing null input
      // Null is passed through to the mock (not converted to {})
      await TauriCommandService.invoke('test', null);
      expect(mockInvoke).toHaveBeenCalled();
    });

    it('should handle undefined arguments', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', undefined);
      expect(mockInvoke).toHaveBeenCalledWith('test', {});
    });

    it('should handle empty object arguments', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', {});
      expect(mockInvoke).toHaveBeenCalledWith('test', {});
    });

    it('should handle arguments with special values', async () => {
      const specialArgs = {
        nullValue: null,
        undefinedValue: undefined,
        emptyString: '',
        zero: 0,
        false: false,
        negative: -1,
        largeNumber: Number.MAX_SAFE_INTEGER,
        specialChars: '!@#$%^&*()',
      };

      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', specialArgs);
      expect(mockInvoke).toHaveBeenCalledWith('test', specialArgs);
    });

    it('should handle deeply nested arguments', async () => {
      const nestedArgs = {
        level1: {
          level2: {
            level3: {
              level4: {
                value: 'deep',
              },
            },
          },
        },
      };

      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', nestedArgs);
      expect(mockInvoke).toHaveBeenCalledWith('test', nestedArgs);
    });

    it('should handle array arguments', async () => {
      const arrayArgs = {
        items: [1, 2, 3, 4, 5],
        nested: [[1, 2], [3, 4]],
        empty: [],
      };

      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', arrayArgs);
      expect(mockInvoke).toHaveBeenCalledWith('test', arrayArgs);
    });
  });

  describe('retry logic edge cases', () => {
    it('should handle zero retries', async () => {
      mockInvoke.mockRejectedValue(new Error('Test error'));

      await expect(
        TauriCommandService.invoke('test', {}, { retry: 0 })
      ).rejects.toThrow(TauriCommandError);

      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    it('should handle negative retry count', async () => {
      mockInvoke.mockRejectedValue(new Error('Test error'));

      // @ts-expect-error - Testing negative input
      await expect(
        TauriCommandService.invoke('test', {}, { retry: -1 })
      ).rejects.toThrow(TauriCommandError);
    });

    it('should handle large retry count', async () => {
      // Succeed immediately to avoid timeout from exponential backoff
      mockInvoke.mockResolvedValue({ success: true });

      const result = await TauriCommandService.invoke('test', {}, { retry: 100 });
      expect(result).toEqual({ success: true });
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    it('should stop retrying on non-retriable errors', async () => {
      mockInvoke.mockRejectedValue(new Error('Validation failed'));

      await expect(
        TauriCommandService.invoke('test', {}, { retry: 5 })
      ).rejects.toThrow(TauriCommandError);

      // Should only try once for validation error
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    it('should retry on transient errors', async () => {
      mockInvoke.mockReset(); // Reset to clear any previous calls
      let attempts = 0;
      mockInvoke.mockImplementation(() => {
        attempts++;
        // Fail first attempt, succeed on second
        if (attempts === 1) {
          throw new Error('Network error');
        }
        return Promise.resolve({ success: true });
      });

      const result = await TauriCommandService.invoke('test', {}, { retry: 3 });
      expect(result).toEqual({ success: true });
      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });
  });

  describe('error handling edge cases', () => {
    it('should handle Error objects', async () => {
      const originalError = new Error('Original error');
      mockInvoke.mockRejectedValue(originalError);

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });

    it('should handle string errors', async () => {
      mockInvoke.mockRejectedValue('String error');

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });

    it('should handle null errors', async () => {
      mockInvoke.mockRejectedValue(null);

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });

    it('should handle undefined errors', async () => {
      mockInvoke.mockRejectedValue(undefined);

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });

    it('should handle errors without message property', async () => {
      mockInvoke.mockRejectedValue({ code: 'ERR_TEST' });

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });

    it('should handle errors with very long messages', async () => {
      const longMessage = 'x'.repeat(100000);
      mockInvoke.mockRejectedValue(new Error(longMessage));

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });

    it('should handle errors with special characters in message', async () => {
      const specialMessage = 'Error: \n\r\t\x00\x1f unicode: \u{1f600}';
      mockInvoke.mockRejectedValue(new Error(specialMessage));

      await expect(TauriCommandService.invoke('test')).rejects.toThrow(TauriCommandError);
    });
  });

  describe('response type edge cases', () => {
    it('should handle null responses', async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await TauriCommandService.invoke('test');
      expect(result).toBeNull();
    });

    it('should handle undefined responses', async () => {
      mockInvoke.mockResolvedValue(undefined);

      const result = await TauriCommandService.invoke('test');
      expect(result).toBeUndefined();
    });

    it('should handle empty string responses', async () => {
      mockInvoke.mockResolvedValue('');

      const result = await TauriCommandService.invoke('test');
      expect(result).toBe('');
    });

    it('should handle zero responses', async () => {
      mockInvoke.mockResolvedValue(0);

      const result = await TauriCommandService.invoke('test');
      expect(result).toBe(0);
    });

    it('should handle false responses', async () => {
      mockInvoke.mockResolvedValue(false);

      const result = await TauriCommandService.invoke('test');
      expect(result).toBe(false);
    });

    it('should handle empty array responses', async () => {
      mockInvoke.mockResolvedValue([]);

      const result = await TauriCommandService.invoke('test');
      expect(result).toEqual([]);
    });

    it('should handle empty object responses', async () => {
      mockInvoke.mockResolvedValue({});

      const result = await TauriCommandService.invoke('test');
      expect(result).toEqual({});
    });

    it('should handle very large responses', async () => {
      const largeArray = new Array(1000000).fill('data');
      mockInvoke.mockResolvedValue(largeArray);

      const result = await TauriCommandService.invoke('test');
      expect(result).toHaveLength(1000000);
    });
  });

  describe('latency tracking edge cases', () => {
    it('should handle negative latency', async () => {
      // Mock performance.now to return negative (unlikely but possible edge case)
      const originalNow = performance.now;
      let callCount = 0;
      vi.spyOn(performance, 'now').mockImplementation(() => {
        callCount++;
        // First call returns positive, second returns negative
        return callCount === 1 ? 100 : -50;
      });

      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test');

      const stats = TauriCommandService.getLatencyStats('test');
      // Should still track even with unusual values
      expect(stats).not.toBeNull();

      vi.spyOn(performance, 'now').mockRestore();
    });

    it('should handle very high latency', async () => {
      vi.spyOn(performance, 'now').mockReturnValue(1e15); // Very large number

      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test');

      const stats = TauriCommandService.getLatencyStats('test');
      expect(stats).not.toBeNull();

      vi.spyOn(performance, 'now').mockRestore();
    });

    it('should handle overflow in latency tracking (100+ calls)', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      // Make 150 calls to test the 100 measurement limit
      for (let i = 0; i < 150; i++) {
        await TauriCommandService.invoke('test');
      }

      const stats = TauriCommandService.getLatencyStats('test');
      expect(stats).not.toBeNull();
      expect(stats?.count).toBeLessThanOrEqual(100);
    });
  });

  describe('safeInvoke edge cases', () => {
    it('should return success result for successful invocation', async () => {
      mockInvoke.mockResolvedValue({ data: 'test' });

      const result = await TauriCommandService.safeInvoke('test');

      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data).toEqual({ data: 'test' });
      }
    });

    it('should return error result for failed invocation', async () => {
      mockInvoke.mockRejectedValue(new Error('Test error'));

      const result = await TauriCommandService.safeInvoke('test');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBeInstanceOf(TauriCommandError);
      }
    });

    it('should handle non-TauriCommandError exceptions', async () => {
      mockInvoke.mockRejectedValue('Random error');

      const result = await TauriCommandService.safeInvoke('test');

      expect(result.success).toBe(false);
      if (!result.success) {
        expect(result.error).toBeInstanceOf(TauriCommandError);
      }
    });
  });

  describe('convenience functions', () => {
    it('should handle invokeCommand with edge cases', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      // @ts-expect-error - Testing null input
      // The mock receives null as args, not converted to {}
      await invokeCommand('test', null);
      expect(mockInvoke).toHaveBeenCalled();
    });

    it('should handle safeInvokeCommand with edge cases', async () => {
      mockInvoke.mockRejectedValue(new Error('Test error'));

      const result = await safeInvokeCommand('test', undefined);

      expect(result.success).toBe(false);
    });
  });

  describe('options edge cases', () => {
    it('should handle undefined options', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', {}, undefined);
      expect(mockInvoke).toHaveBeenCalledWith('test', {});
    });

    it('should handle null options', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      // @ts-expect-error - Testing null input
      await TauriCommandService.invoke('test', {}, null);
      expect(mockInvoke).toHaveBeenCalledWith('test', {});
    });

    it('should handle empty options', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', {}, {});
      expect(mockInvoke).toHaveBeenCalledWith('test', {});
    });

    it('should handle partial options', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      await TauriCommandService.invoke('test', {}, { retry: 5 });
      expect(mockInvoke).toHaveBeenCalledWith('test', {});

      await TauriCommandService.invoke('test', {}, { timeout: 1000 });
      expect(mockInvoke).toHaveBeenCalledWith('test', {});

      await TauriCommandService.invoke('test', {}, { log: false });
      expect(mockInvoke).toHaveBeenCalledWith('test', {});
    });

    it('should handle invalid option values', async () => {
      mockInvoke.mockResolvedValue({ success: true });

      // Test that invalid options don't cause crashes
      // The implementation should handle or ignore them gracefully
      // @ts-expect-error - Testing invalid inputs
      await TauriCommandService.invoke('test', {}, { log: 'yes' as unknown as boolean });
      // @ts-expect-error - Testing invalid inputs
      await TauriCommandService.invoke('test', {}, { timeout: -100 });

      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });
  });

  describe('TauriCommandError edge cases', () => {
    it('should handle empty command name', () => {
      const error = new TauriCommandError('', 'Test message');
      expect(error.message).toContain('[Tauri:]');
      expect(error.command).toBe('');
    });

    it('should handle very long command name', () => {
      const longCommand = 'x'.repeat(10000);
      const error = new TauriCommandError(longCommand, 'Test message');
      expect(error.command).toBe(longCommand);
    });

    it('should handle empty error message', () => {
      const error = new TauriCommandError('test', '');
      expect(error.message).toContain('[Tauri:test]');
      // Message is just the prefix when error message is empty
    });

    it('should handle special characters in error message', () => {
      const specialMessage = 'Error: \n\r\t\x00\x1f';
      const error = new TauriCommandError('test', specialMessage);
      expect(error.message).toContain('Tauri:test]');
      // Note: special characters may be escaped in the error message
    });

    it('should handle null original error', () => {
      const error = new TauriCommandError('test', 'Test message', null);
      expect(error.originalError).toBeNull();
    });

    it('should handle undefined original error', () => {
      const error = new TauriCommandError('test', 'Test message', undefined);
      expect(error.originalError).toBeUndefined();
    });
  });
});
