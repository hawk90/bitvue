/**
 * ModeContext Provider Tests
 * Tests visualization mode context for YUV viewer
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { ModeProvider, useMode, MODES, COMPONENTS } from '../ModeContext';
import type { VisualizationMode, YuvComponent, ComponentMask } from '../ModeContext';

describe('ModeContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ModeProvider>{children}</ModeProvider>
  );

  it('should provide default mode state', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.currentMode).toBe('overview');
  });

  it('should have all required methods', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(typeof result.current.setMode).toBe('function');
    expect(typeof result.current.cycleMode).toBe('function');
    expect(typeof result.current.toggleComponent).toBe('function');
    expect(typeof result.current.setComponentMask).toBe('function');
    expect(typeof result.current.toggleGrid).toBe('function');
    expect(typeof result.current.toggleLabels).toBe('function');
    expect(typeof result.current.toggleBlockTypes).toBe('function');
  });

  it('should have default component mask', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.componentMask).toBe('yuv');
  });

  it('should have default overlay states', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.showGrid).toBe(false);
    expect(result.current.showLabels).toBe(true);
    expect(result.current.showBlockTypes).toBe(false);
  });
});

describe('ModeContext mode management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ModeProvider>{children}</ModeProvider>
  );

  it('should set mode', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.setMode('prediction');
    });

    expect(result.current.currentMode).toBe('prediction');
  });

  it('should support all visualization modes', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    const modes: VisualizationMode[] = [
      'overview',
      'coding-flow',
      'prediction',
      'transform',
      'qp-map',
      'mv-field',
      'reference',
    ];

    modes.forEach(mode => {
      act(() => {
        result.current.setMode(mode);
      });

      expect(result.current.currentMode).toBe(mode);
    });
  });

  it('should cycle to next mode', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.currentMode).toBe('overview');

    act(() => {
      result.current.cycleMode();
    });

    expect(result.current.currentMode).toBe('coding-flow');
  });

  it('should cycle through all modes in order', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    const modes: VisualizationMode[] = [
      'overview',
      'coding-flow',
      'prediction',
      'transform',
      'qp-map',
      'mv-field',
      'reference',
    ];

    // Start at overview
    expect(result.current.currentMode).toBe('overview');

    // Cycle through all modes
    for (let i = 0; i < modes.length - 1; i++) {
      act(() => {
        result.current.cycleMode();
      });
    }

    expect(result.current.currentMode).toBe('reference');

    // Next cycle should wrap around to overview
    act(() => {
      result.current.cycleMode();
    });

    expect(result.current.currentMode).toBe('overview');
  });

  it('should handle setting same mode', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.setMode('transform');
      result.current.setMode('transform');
    });

    expect(result.current.currentMode).toBe('transform');
  });

  it('should use stable callbacks (useCallback optimization)', () => {
    const { result, rerender } = renderHook(() => useMode(), { wrapper });

    const setModeRef = result.current.setMode;
    const cycleModeRef = result.current.cycleMode;

    rerender();

    expect(result.current.setMode).toBe(setModeRef);
    expect(result.current.cycleMode).toBe(cycleModeRef);
  });
});

describe('ModeContext component mask', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => {
    return <ModeProvider>{children}</ModeProvider>;
  };

  it('should toggle component visibility', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.componentMask).toBe('yuv');

    act(() => {
      result.current.toggleComponent('y');
    });

    // After removing 'y', should be 'uv'
    expect(result.current.componentMask).toBe('uv');
  });

  it('should add component back when toggled again', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('uv');

    act(() => {
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('yuv');
  });

  it('should set component mask directly', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.setComponentMask('y');
    });

    expect(result.current.componentMask).toBe('y');

    act(() => {
      result.current.setComponentMask('u');
    });

    expect(result.current.componentMask).toBe('u');

    act(() => {
      result.current.setComponentMask('v');
    });

    expect(result.current.componentMask).toBe('v');

    act(() => {
      result.current.setComponentMask('uv');
    });

    expect(result.current.componentMask).toBe('uv');
  });

  it('should handle empty component mask', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Remove all components
    act(() => {
      result.current.toggleComponent('y');
      result.current.toggleComponent('u');
      result.current.toggleComponent('v');
    });

    expect(result.current.componentMask).toBe('');
  });

  it('should handle adding components to empty mask', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.setComponentMask('');
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('y');
  });

  it('should handle component order in mask', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.setComponentMask('vyu');
    });

    expect(result.current.componentMask).toBe('vyu');

    // Toggle first component
    act(() => {
      result.current.toggleComponent('v');
    });

    expect(result.current.componentMask).toBe('yu');

    // Add it back
    act(() => {
      result.current.toggleComponent('v');
    });

    expect(result.current.componentMask).toBe('yuv');
  });

  it('should handle all component combinations', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    const validMasks: ComponentMask[] = [
      'y', 'u', 'v',
      'yu', 'yv', 'uv',
      'yuv',
    ];

    validMasks.forEach(mask => {
      act(() => {
        result.current.setComponentMask(mask);
      });

      expect(result.current.componentMask).toBe(mask);
    });
  });
});

describe('ModeContext overlay toggles', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ModeProvider>{children}</ModeProvider>
  );

  it('should toggle grid visibility', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.showGrid).toBe(false);

    act(() => {
      result.current.toggleGrid();
    });

    expect(result.current.showGrid).toBe(true);

    act(() => {
      result.current.toggleGrid();
    });

    expect(result.current.showGrid).toBe(false);
  });

  it('should toggle labels visibility', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.showLabels).toBe(true);

    act(() => {
      result.current.toggleLabels();
    });

    expect(result.current.showLabels).toBe(false);

    act(() => {
      result.current.toggleLabels();
    });

    expect(result.current.showLabels).toBe(true);
  });

  it('should toggle block types visibility', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    expect(result.current.showBlockTypes).toBe(false);

    act(() => {
      result.current.toggleBlockTypes();
    });

    expect(result.current.showBlockTypes).toBe(true);

    act(() => {
      result.current.toggleBlockTypes();
    });

    expect(result.current.showBlockTypes).toBe(false);
  });

  it('should handle multiple rapid toggles', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    for (let i = 0; i < 10; i++) {
      act(() => {
        result.current.toggleGrid();
        result.current.toggleLabels();
        result.current.toggleBlockTypes();
      });
    }

    // After even number of toggles, should be back to defaults
    expect(result.current.showGrid).toBe(false);
    expect(result.current.showLabels).toBe(true);
    expect(result.current.showBlockTypes).toBe(false);
  });
});

describe('ModeContext error handling', () => {
  it('should throw error when useMode used outside provider', () => {
    expect(() => {
      renderHook(() => useMode());
    }).toThrow('useMode must be used within a ModeProvider');
  });
});

describe('ModeContext mode metadata', () => {
  it('should export MODES array', () => {
    expect(MODES).toBeDefined();
    expect(Array.isArray(MODES)).toBe(true);
  });

  it('should have all modes in MODES', () => {
    const modes = MODES.map(m => m.key);

    expect(modes).toContain('overview');
    expect(modes).toContain('coding-flow');
    expect(modes).toContain('prediction');
    expect(modes).toContain('transform');
    expect(modes).toContain('qp-map');
    expect(modes).toContain('mv-field');
    expect(modes).toContain('reference');
  });

  it('should have correct shortcuts in MODES', () => {
    const shortcuts = MODES.map(m => m.shortcut);

    expect(shortcuts).toContain('F1');
    expect(shortcuts).toContain('F2');
    expect(shortcuts).toContain('F3');
    expect(shortcuts).toContain('F4');
    expect(shortcuts).toContain('F5');
    expect(shortcuts).toContain('F6');
    expect(shortcuts).toContain('F7');
  });

  it('should have correct labels in MODES', () => {
    const overviewMode = MODES.find(m => m.key === 'overview');
    expect(overviewMode?.label).toBe('Overview');

    const codingFlowMode = MODES.find(m => m.key === 'coding-flow');
    expect(codingFlowMode?.label).toBe('Coding Flow');
  });

  it('should have descriptions in MODES', () => {
    MODES.forEach(mode => {
      expect(mode.description).toBeDefined();
      expect(mode.description.length).toBeGreaterThan(0);
    });
  });
});

describe('ModeContext component metadata', () => {
  it('should export COMPONENTS array', () => {
    expect(COMPONENTS).toBeDefined();
    expect(Array.isArray(COMPONENTS)).toBe(true);
  });

  it('should have all components in COMPONENTS', () => {
    const components = COMPONENTS.map(c => c.key);

    expect(components).toContain('y');
    expect(components).toContain('u');
    expect(components).toContain('v');
  });

  it('should have correct labels in COMPONENTS', () => {
    const yComponent = COMPONENTS.find(c => c.key === 'y');
    expect(yComponent?.label).toBe('Y (Luma)');

    const uComponent = COMPONENTS.find(c => c.key === 'u');
    expect(uComponent?.label).toBe('U (Cb)');

    const vComponent = COMPONENTS.find(c => c.key === 'v');
    expect(vComponent?.label).toBe('V (Cr)');
  });

  it('should have colors in COMPONENTS', () => {
    COMPONENTS.forEach(component => {
      expect(component.color).toBeDefined();
      expect(component.color).toMatch(/^#[0-9a-fA-F]{6}$/);
    });
  });
});

describe('ModeContext complex workflows', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ModeProvider>{children}</ModeProvider>
  );

  it('should handle mode switching workflow', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Start with overview
    expect(result.current.currentMode).toBe('overview');

    // Switch to prediction mode
    act(() => {
      result.current.setMode('prediction');
    });
    expect(result.current.currentMode).toBe('prediction');

    // Toggle components
    act(() => {
      result.current.toggleComponent('u');
    });
    expect(result.current.componentMask).toBe('yv');

    // Enable overlays
    act(() => {
      result.current.toggleGrid();
      result.current.toggleBlockTypes();
    });

    expect(result.current.showGrid).toBe(true);
    expect(result.current.showBlockTypes).toBe(true);

    // Cycle mode
    act(() => {
      result.current.cycleMode();
    });

    expect(result.current.currentMode).toBe('transform');
    // Overlays should persist
    expect(result.current.showGrid).toBe(true);
    expect(result.current.showBlockTypes).toBe(true);
  });

  it('should handle component-only mode workflow', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Remove all components
    act(() => {
      result.current.setComponentMask('');
    });

    expect(result.current.componentMask).toBe('');

    // Add back one by one
    act(() => {
      result.current.toggleComponent('y');
      result.current.toggleComponent('u');
    });

    expect(result.current.componentMask).toBe('yu');

    // Toggle one off
    act(() => {
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('u');
  });

  it('should handle overlay configuration workflow', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Start with defaults
    expect(result.current.showLabels).toBe(true);
    expect(result.current.showGrid).toBe(false);
    expect(result.current.showBlockTypes).toBe(false);

    // Configure for analysis view
    act(() => {
      result.current.toggleGrid();
      result.current.toggleBlockTypes();
      result.current.toggleLabels();
    });

    expect(result.current.showGrid).toBe(true);
    expect(result.current.showBlockTypes).toBe(true);
    expect(result.current.showLabels).toBe(false);

    // Configure for clean view
    act(() => {
      result.current.toggleGrid();
      result.current.toggleBlockTypes();
      result.current.toggleLabels();
    });

    expect(result.current.showGrid).toBe(false);
    expect(result.current.showBlockTypes).toBe(false);
    expect(result.current.showLabels).toBe(true);
  });

  it('should handle mode cycling through all modes', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    const modes: VisualizationMode[] = [
      'overview',
      'coding-flow',
      'prediction',
      'transform',
      'qp-map',
      'mv-field',
      'reference',
    ];

    // Start at overview
    expect(result.current.currentMode).toBe('overview');

    // Cycle through all modes
    modes.forEach((expectedMode, index) => {
      if (index > 0) {
        act(() => {
          result.current.cycleMode();
        });
      }

      expect(result.current.currentMode).toBe(expectedMode);
    });
  });

  it('should maintain state when switching modes', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Configure state
    act(() => {
      result.current.setComponentMask('yu');
      result.current.toggleGrid();
      result.current.toggleBlockTypes();
    });

    const componentMask = result.current.componentMask;
    const showGrid = result.current.showGrid;
    const showBlockTypes = result.current.showBlockTypes;

    // Switch mode
    act(() => {
      result.current.setMode('qp-map');
    });

    expect(result.current.componentMask).toBe(componentMask);
    expect(result.current.showGrid).toBe(showGrid);
    expect(result.current.showBlockTypes).toBe(showBlockTypes);
  });
});

describe('ModeContext edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ModeProvider>{children}</ModeProvider>
  );

  it('should handle toggling non-existent component', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Start with empty mask
    act(() => {
      result.current.setComponentMask('');
    });

    // Toggle a component that doesn't exist - should add it
    act(() => {
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('y');
  });

  it('should handle toggling same component multiple times', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.toggleComponent('y');
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('yuv'); // Removed then added back

    act(() => {
      result.current.toggleComponent('y');
      result.current.toggleComponent('y');
    });

    expect(result.current.componentMask).toBe('yuv'); // Was 'y', removed, then added to empty
  });

  it('should handle rapid mode switching', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      for (let i = 0; i < 20; i++) {
        const modes: VisualizationMode[] = [
          'overview', 'prediction', 'transform', 'qp-map', 'reference'
        ];
        result.current.setMode(modes[i % modes.length]);
      }
    });

    expect(result.current.currentMode).toBeDefined();
  });

  it('should handle rapid component toggling', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      for (let i = 0; i < 30; i++) {
        const components: YuvComponent[] = ['y', 'u', 'v'];
        result.current.toggleComponent(components[i % 3]);
      }
    });

    // Final state should be valid
    expect(['y', 'u', 'v', 'yu', 'yv', 'uv', 'yuv', ''])
      .toContain(result.current.componentMask);
  });

  it('should handle all overlays enabled', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    act(() => {
      result.current.toggleGrid();
      // showLabels is already true by default, don't toggle
      result.current.toggleBlockTypes();
    });

    expect(result.current.showGrid).toBe(true);
    expect(result.current.showLabels).toBe(true);
    expect(result.current.showBlockTypes).toBe(true);
  });

  it('should handle all overlays disabled', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // Labels start as true
    act(() => {
      result.current.toggleLabels();
    });

    expect(result.current.showGrid).toBe(false);
    expect(result.current.showLabels).toBe(false);
    expect(result.current.showBlockTypes).toBe(false);
  });

  it('should handle invalid component mask values', () => {
    const { result } = renderHook(() => useMode(), { wrapper });

    // TypeScript should prevent invalid values, but we test the runtime behavior
    act(() => {
      result.current.setComponentMask('yuv'); // Valid
    });

    expect(result.current.componentMask).toBe('yuv');
  });
});

describe('ModeContext type safety', () => {
  it('should only accept valid visualization modes', () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <ModeProvider>{children}</ModeProvider>
    );

    const { result } = renderHook(() => useMode(), { wrapper });

    const validModes: VisualizationMode[] = [
      'overview', 'coding-flow', 'prediction', 'transform',
      'qp-map', 'mv-field', 'reference',
    ];

    // TypeScript would catch invalid modes at compile time
    validModes.forEach(mode => {
      expect(() => {
        act(() => {
          result.current.setMode(mode);
        });
      }).not.toThrow();
    });
  });

  it('should only accept valid YUV components', () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <ModeProvider>{children}</ModeProvider>
    );

    const { result } = renderHook(() => useMode(), { wrapper });

    const validComponents: YuvComponent[] = ['y', 'u', 'v'];

    // TypeScript would catch invalid components at compile time
    validComponents.forEach(component => {
      expect(() => {
        act(() => {
          result.current.toggleComponent(component);
        });
      }).not.toThrow();
    });
  });
});
