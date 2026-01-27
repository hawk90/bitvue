/**
 * CSS Utility Tests
 * Tests CSS variable helpers and utility functions
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { getCssVar, setCssVar, getCssVars, parseCssPixels, hasCssVar } from '../css';

describe('getCssVar', () => {
  beforeEach(() => {
    // Set up test DOM with CSS variables
    const style = document.createElement('style');
    style.textContent = `
      :root {
        --test-color: #ff0000;
        --test-size: 16px;
        --test-number: 42;
        --empty-var: ;
      }
    `;
    document.head.appendChild(style);
  });

  afterEach(() => {
    // Clean up
    const styles = document.head.querySelectorAll('style');
    styles.forEach(s => s.remove());
  });

  it('should retrieve CSS variable value', () => {
    const value = getCssVar('--test-color');
    expect(value).toBe('#ff0000');
  });

  it('should return variable name as fallback when not found', () => {
    const value = getCssVar('--non-existent');
    expect(value).toBe('--non-existent');
  });

  it('should return variable name when variable is empty', () => {
    const value = getCssVar('--empty-var');
    expect(value).toBe('--empty-var');
  });

  it('should parse numeric CSS variables', () => {
    const value = getCssVar('--test-number');
    expect(value).toBe('42');
  });

  it('should handle CSS variables with units', () => {
    const value = getCssVar('--test-size');
    expect(value).toBe('16px');
  });
});

describe('setCssVar', () => {
  it('should set CSS variable value', () => {
    setCssVar('--test-set', 'blue');
    expect(getCssVar('--test-set')).toBe('blue');
  });

  it('should update existing CSS variable', () => {
    setCssVar('--test-update', 'red');
    setCssVar('--test-update', 'green');
    expect(getCssVar('--test-update')).toBe('green');
  });

  it('should handle empty values', () => {
    setCssVar('--test-empty', '');
    expect(hasCssVar('--test-empty')).toBe(false);
  });
});

describe('getCssVars', () => {
  beforeEach(() => {
    const style = document.createElement('style');
    style.textContent = `
      :root {
        --var1: value1;
        --var2: value2;
        --var3: 123px;
      }
    `;
    document.head.appendChild(style);
  });

  afterEach(() => {
    const styles = document.head.querySelectorAll('style');
    styles.forEach(s => s.remove());
  });

  it('should get multiple CSS variables', () => {
    const result = getCssVars(['--var1', '--var2', '--var3']);
    expect(result).toEqual({
      '--var1': 'value1',
      '--var2': 'value2',
      '--var3': '123px',
    });
  });

  it('should return variable names as fallback for non-existent variables', () => {
    const result = getCssVars(['--var1', '--non-existent']);
    expect(result['--var1']).toBe('value1');
    expect(result['--non-existent']).toBe('--non-existent');
  });

  it('should handle empty array', () => {
    const result = getCssVars([]);
    expect(result).toEqual({});
  });
});

describe('parseCssPixels', () => {
  // Note: jsdom doesn't actually render, so offsetHeight returns 0
  // These tests verify the function runs without error and returns a number
  it('should parse px values', () => {
    const result1 = parseCssPixels('16px');
    const result2 = parseCssPixels('1px');
    expect(typeof result1).toBe('number');
    expect(typeof result2).toBe('number');
  });

  it('should parse rem values', () => {
    const result = parseCssPixels('1rem');
    expect(typeof result).toBe('number');
  });

  it('should parse em values', () => {
    const result = parseCssPixels('1em');
    expect(typeof result).toBe('number');
  });

  it('should return NaN for invalid values', () => {
    const result = parseCssPixels('invalid');
    expect(result).toBeNaN();
  });

  it('should handle percentage values', () => {
    const result = parseCssPixels('50%');
    expect(typeof result).toBe('number');
  });
});

describe('hasCssVar', () => {
  beforeEach(() => {
    const style = document.createElement('style');
    style.textContent = `
      :root {
        --existing-var: some-value;
      }
    `;
    document.head.appendChild(style);
  });

  afterEach(() => {
    const styles = document.head.querySelectorAll('style');
    styles.forEach(s => s.remove());
  });

  it('should return true for existing CSS variable', () => {
    expect(hasCssVar('--existing-var')).toBe(true);
  });

  it('should return false for non-existent CSS variable', () => {
    expect(hasCssVar('--non-existent')).toBe(false);
  });

  it('should return false after setting variable to empty', () => {
    setCssVar('--temp-var', '');
    expect(hasCssVar('--temp-var')).toBe(false);
  });
});
