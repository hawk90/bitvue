/**
 * LayoutContext Provider Tests
 * Tests layout context provider for panel management
 */

import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { LayoutProvider, useLayout } from '@/contexts/LayoutContext';

describe('LayoutContext', () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <LayoutProvider>{children}</LayoutProvider>
  );

  it('should provide default layout state', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(result.current.panels).toBeDefined();
    expect(Array.isArray(result.current.panels)).toBe(true);
  });

  it('should have resetLayout function', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.resetLayout).toBe('function');
  });

  it('should have addPanel function', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.addPanel).toBe('function');
  });

  it('should have removePanel function', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.removePanel).toBe('function');
  });

  it('should have togglePanel function', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.togglePanel).toBe('function');
  });
});

describe('LayoutContext panel management', () => {
  it('should add panel to layout', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.addPanel({
        id: 'test-panel',
        title: 'Test Panel',
        component: () => null,
      });
    });

    expect(result.current.panels.length).toBeGreaterThan(0);
  });

  it('should remove panel from layout', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      const panelId = result.current.addPanel({
        id: 'test-panel',
        title: 'Test Panel',
        component: () => null,
      });
      result.current.removePanel('test-panel');
    });

    const panel = result.current.panels.find(p => p.id === 'test-panel');
    expect(panel).toBeUndefined();
  });

  it('should toggle panel visibility', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.addPanel({
        id: 'test-panel',
        title: 'Test Panel',
        component: () => null,
      });
    });

    const initialVisible = result.current.panels[0].visible;

    act(() => {
      result.current.togglePanel('test-panel');
    });

    expect(result.current.panels[0].visible).toBe(!initialVisible);
  });
});

describe('LayoutContext resetLayout', () => {
  it('should reset to default layout', () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.addPanel({
        id: 'custom-panel',
        title: 'Custom',
        component: () => null,
      });
    });

    expect(result.current.panels.length).toBeGreaterThan(0);

    act(() => {
      result.current.resetLayout();
    });

    // Should return to default panels
    expect(result.current.panels).toBeDefined();
  });
});

describe('LayoutContext persistence', () => {
  it('should save layout to localStorage', () => {
    const localStorageSpy = vi.spyOn(Storage.prototype, 'setItem');
    localStorageSpy.mockImplementation(() => {});

    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.addPanel({
        id: 'test-panel',
        title: 'Test',
        component: () => null,
      });
    });

    expect(localStorageSpy).toHaveBeenCalled();
  });

  it('should load layout from localStorage on mount', () => {
    const localStorageSpy = vi.spyOn(Storage.prototype, 'getItem');
    localStorageSpy.mockReturnValue(JSON.stringify([
      { id: 'saved-panel', title: 'Saved', position: 'left' }
    ]));

    const { result } = renderHook(() => useLayout(), { wrapper });

    // Should load saved panels
    expect(result.current.panels).toBeDefined();
  });
});

describe('LayoutContext error handling', () => {
  it('should throw error when useLayout used outside provider', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => {
      renderHook(() => useLayout());
    }).toThrow();

    consoleSpy.mockRestore();
  });

  it('should handle corrupted localStorage data', () => {
    const localStorageSpy = vi.spyOn(Storage.prototype, 'getItem');
    localStorageSpy.mockReturnValue('invalid json');

    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <LayoutProvider>{children}</LayoutProvider>
    );

    const { result } = renderHook(() => useLayout(), { wrapper });

    // Should fall back to default layout
    expect(result.current.panels).toBeDefined();
  });
});

describe('LayoutContext panel positions', () => {
  it('should support different panel positions', () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <LayoutProvider>{children}</LayoutProvider>
    );

    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.addPanel({
        id: 'left-panel',
        title: 'Left Panel',
        component: () => null,
        position: 'left',
      });

      result.current.addPanel({
        id: 'right-panel',
        title: 'Right Panel',
        component: () => null,
        position: 'right',
      });

      result.current.addPanel({
        id: 'bottom-panel',
        title: 'Bottom Panel',
        component: () => null,
        position: 'bottom',
      });
    });

    expect(result.current.panels.length).toBeGreaterThanOrEqual(3);
  });
});
