/**
 * Array Utility Tests
 * Tests array manipulation utilities
 */

import { describe, it, expect } from 'vitest';

describe('chunk', () => {
  const chunk = <T>(array: T[], size: number): T[][] => {
    const chunks: T[][] = [];
    for (let i = 0; i < array.length; i += size) {
      chunks.push(array.slice(i, i + size));
    }
    return chunks;
  };

  it('should chunk array into groups', () => {
    expect(chunk([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
    expect(chunk([1, 2, 3, 4, 5, 6], 3)).toEqual([[1, 2, 3], [4, 5, 6]]);
  });

  it('should handle chunk size larger than array', () => {
    expect(chunk([1, 2, 3], 5)).toEqual([[1, 2, 3]]);
  });

  it('should handle empty array', () => {
    expect(chunk([], 2)).toEqual([]);
  });

  it('should handle chunk size of 1', () => {
    expect(chunk([1, 2, 3], 1)).toEqual([[1], [2], [3]]);
  });
});

describe('unique', () => {
  const unique = <T>(array: T[]): T[] => {
    return Array.from(new Set(array));
  };

  it('should remove duplicates', () => {
    expect(unique([1, 2, 2, 3, 3, 3])).toEqual([1, 2, 3]);
    expect(unique(['a', 'b', 'a', 'c'])).toEqual(['a', 'b', 'c']);
  });

  it('should handle empty array', () => {
    expect(unique([])).toEqual([]);
  });

  it('should handle array with no duplicates', () => {
    expect(unique([1, 2, 3])).toEqual([1, 2, 3]);
  });
});

describe('groupBy', () => {
  const groupBy = <T>(array: T[], keyFn: (item: T) => string): Record<string, T[]> => {
    return array.reduce((groups, item) => {
      const key = keyFn(item);
      (groups[key] = groups[key] || []).push(item);
      return groups;
    }, {} as Record<string, T[]>);
  };

  it('should group by key function', () => {
    const items = [
      { type: 'fruit', name: 'apple' },
      { type: 'fruit', name: 'banana' },
      { type: 'vegetable', name: 'carrot' },
    ];

    const grouped = groupBy(items, item => item.type);

    expect(grouped.fruit).toHaveLength(2);
    expect(grouped.vegetable).toHaveLength(1);
  });

  it('should handle empty array', () => {
    expect(groupBy([], (x: any) => x)).toEqual({});
  });
});

describe('shuffle', () => {
  const shuffle = <T>(array: T[]): T[] => {
    const result = [...array];
    for (let i = result.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [result[i], result[j]] = [result[j], result[i]];
    }
    return result;
  };

  it('should return array of same length', () => {
    const input = [1, 2, 3, 4, 5];
    const result = shuffle(input);

    expect(result).toHaveLength(5);
    expect(result).toContain(1);
    expect(result).toContain(5);
  });

  it('should not mutate original array', () => {
    const input = [1, 2, 3];
    const original = [...input];
    shuffle(input);

    expect(input).toEqual(original);
  });
});

describe('flatten', () => {
  const flatten = <T>(array: (T | T[])[][]): T[] => {
    return array.flat(2) as T[];
  };

  it('should flatten nested arrays', () => {
    expect(flatten([[1, [2, 3]], 4, [[5]]])).toEqual([1, 2, 3, 4, 5]);
    expect(flatten([[[[1]]]])).toEqual([1]);
  });

  it('should handle empty arrays', () => {
    expect(flatten([[], [[]]])).toEqual([]);
    expect(flatten([])).toEqual([]);
  });
});

describe('sum', () => {
  const sum = (array: number[]): number => {
    return array.reduce((acc, val) => acc + val, 0);
  };

  it('should sum numbers', () => {
    expect(sum([1, 2, 3, 4, 5])).toBe(15);
    expect(sum([10, -5, 5])).toBe(10);
  });

  it('should return 0 for empty array', () => {
    expect(sum([])).toBe(0);
  });

  it('should handle single element', () => {
    expect(sum([42])).toBe(42);
  });
});

describe('average', () => {
  const average = (array: number[]): number => {
    if (array.length === 0) return 0;
    return array.reduce((acc, val) => acc + val, 0) / array.length;
  };

  it('should calculate average', () => {
    expect(average([1, 2, 3, 4, 5])).toBe(3);
    expect(average([10, 20])).toBe(15);
  });

  it('should return 0 for empty array', () => {
    expect(average([])).toBe(0);
  });

  it('should handle negative numbers', () => {
    expect(average([-5, 5])).toBe(0);
    expect(average([-10, 0, 10])).toBe(0);
  });
});

describe('max', () => {
  const max = (array: number[]): number | undefined => {
    return array.reduce((acc, val) => (acc === undefined || val > acc ? val : acc), undefined);
  };

  it('should find maximum', () => {
    expect(max([1, 5, 3, 9, 2])).toBe(9);
    expect(max([-5, -2, -10])).toBe(-2);
  });

  it('should return undefined for empty array', () => {
    expect(max([])).toBeUndefined();
  });

  it('should handle single element', () => {
    expect(max([42])).toBe(42);
  });
});

describe('min', () => {
  const min = (array: number[]): number | undefined => {
    return array.reduce((acc, val) => (acc === undefined || val < acc ? val : acc), undefined);
  };

  it('should find minimum', () => {
    expect(min([1, 5, 3, 9, 2])).toBe(1);
    expect(min([-5, -2, -10])).toBe(-10);
  });

  it('should return undefined for empty array', () => {
    expect(min([])).toBeUndefined();
  });

  it('should handle single element', () => {
    expect(min([42])).toBe(42);
  });
});
