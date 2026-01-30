/**
 * LRU Cache - v0.8.x Memory Management
 *
 * Least Recently Used cache for efficient memory management.
 * Automatically evicts old entries when capacity is reached.
 */

export interface LRUCacheOptions<K, V> {
  /** Maximum number of entries */
  maxSize: number;
  /** Estimate size of value (for memory-based eviction) */
  sizeEstimator?: (value: V) => number;
  /** Maximum memory in bytes (if using sizeEstimator) */
  maxMemory?: number;
  /** Callback when entry is evicted */
  onEvict?: (key: K, value: V) => void;
}

/**
 * LRU Cache node
 */
interface CacheNode<K, V> {
  key: K;
  value: V;
  size: number;
  prev: CacheNode<K, V> | null;
  next: CacheNode<K, V> | null;
}

/**
 * LRU Cache implementation
 */
export class LRUCache<K, V> {
  private cache: Map<K, CacheNode<K, V>>;
  private head: CacheNode<K, V> | null;
  private tail: CacheNode<K, V> | null;
  private maxSize: number;
  private maxMemory: number | null;
  private currentMemory: number;
  private sizeEstimator: ((value: V) => number) | null;
  private onEvict: ((key: K, value: V) => void) | null;

  constructor(options: LRUCacheOptions<K, V>) {
    this.cache = new Map();
    this.head = null;
    this.tail = null;
    this.maxSize = options.maxSize;
    this.maxMemory = options.maxMemory ?? null;
    this.currentMemory = 0;
    this.sizeEstimator = options.sizeEstimator ?? null;
    this.onEvict = options.onEvict ?? null;
  }

  /**
   * Get a value from the cache
   */
  get(key: K): V | undefined {
    const node = this.cache.get(key);
    if (!node) return undefined;

    // Move to head (most recently used)
    this.moveToHead(node);
    return node.value;
  }

  /**
   * Set a value in the cache
   */
  set(key: K, value: V): void {
    const existingNode = this.cache.get(key);

    if (existingNode) {
      // Update existing node
      this.removeFromMemory(existingNode);
      existingNode.value = value;
      existingNode.size = this.estimateSize(value);
      this.addToMemory(existingNode);
      this.moveToHead(existingNode);
    } else {
      // Create new node
      const node: CacheNode<K, V> = {
        key,
        value,
        size: this.estimateSize(value),
        prev: null,
        next: null,
      };

      this.cache.set(key, node);
      this.addToMemory(node);
      this.addToHead(node);

      // Evict if necessary
      this.evictIfNeeded();
    }
  }

  /**
   * Check if key exists
   */
  has(key: K): boolean {
    return this.cache.has(key);
  }

  /**
   * Delete a key from the cache
   */
  delete(key: K): boolean {
    const node = this.cache.get(key);
    if (!node) return false;

    this.removeNode(node);
    this.cache.delete(key);
    return true;
  }

  /**
   * Clear all entries
   */
  clear(): void {
    this.cache.clear();
    this.head = null;
    this.tail = null;
    this.currentMemory = 0;
  }

  /**
   * Get current size (number of entries)
   */
  get size(): number {
    return this.cache.size;
  }

  /**
   * Get current memory usage (in bytes)
   */
  get memoryUsage(): number {
    return this.currentMemory;
  }

  /**
   * Get all keys
   */
  keys(): K[] {
    return Array.from(this.cache.keys());
  }

  /**
   * Get all values
   */
  values(): V[] {
    return Array.from(this.cache.values()).map(node => node.value);
  }

  /**
   * Resize the cache
   */
  resize(newMaxSize: number): void {
    this.maxSize = newMaxSize;
    this.evictIfNeeded();
  }

  /**
   * Estimate size of a value
   */
  private estimateSize(value: V): number {
    if (this.sizeEstimator) {
      return this.sizeEstimator(value);
    }
    return 1;
  }

  /**
   * Add node to memory tracking
   */
  private addToMemory(node: CacheNode<K, V>): void {
    this.currentMemory += node.size;
  }

  /**
   * Remove node from memory tracking
   */
  private removeFromMemory(node: CacheNode<K, V>): void {
    this.currentMemory -= node.size;
  }

  /**
   * Add node to head of list (most recently used)
   */
  private addToHead(node: CacheNode<K, V>): void {
    node.prev = null;
    node.next = this.head;

    if (this.head) {
      this.head.prev = node;
    }
    this.head = node;

    if (!this.tail) {
      this.tail = node;
    }
  }

  /**
   * Move node to head (most recently used)
   */
  private moveToHead(node: CacheNode<K, V>): void {
    if (node === this.head) return;

    this.removeNode(node);
    this.addToHead(node);
  }

  /**
   * Remove node from list
   */
  private removeNode(node: CacheNode<K, V>): void {
    if (node.prev) {
      node.prev.next = node.next;
    } else {
      this.head = node.next;
    }

    if (node.next) {
      node.next.prev = node.prev;
    } else {
      this.tail = node.prev;
    }
  }

  /**
   * Evict least recently used entries if necessary
   */
  private evictIfNeeded(): void {
    // Evict based on count
    while (this.cache.size > this.maxSize) {
      this.evictLRU();
    }

    // Evict based on memory
    if (this.maxMemory !== null) {
      while (this.currentMemory > this.maxMemory && this.tail) {
        this.evictLRU();
      }
    }
  }

  /**
   * Evict least recently used entry
   */
  private evictLRU(): void {
    if (!this.tail) return;

    const lru = this.tail;
    this.removeNode(lru);
    this.cache.delete(lru.key);
    this.currentMemory -= lru.size;

    // Call eviction callback
    if (this.onEvict) {
      this.onEvict(lru.key, lru.value);
    }
  }
}

/**
 * Frame data cache with memory management
 */
export class FrameDataCache {
  private cache: LRUCache<number, FrameDataEntry>;

  constructor(maxFrames: number = 100, maxMemoryMB: number = 500) {
    this.cache = new LRUCache<number, FrameDataEntry>({
      maxSize: maxFrames,
      maxMemory: maxMemoryMB * 1024 * 1024,
      sizeEstimator: (entry) => entry.size,
      onEvict: (key, value) => {
        // Revoke object URLs to free memory
        if (value.thumbnailUrl && value.thumbnailUrl.startsWith('blob:')) {
          URL.revokeObjectURL(value.thumbnailUrl);
        }
      },
    });
  }

  /**
   * Get frame data
   */
  get(frameIndex: number): FrameDataEntry | undefined {
    return this.cache.get(frameIndex);
  }

  /**
   * Set frame data
   */
  set(frameIndex: number, data: Omit<FrameDataEntry, 'size'>): void {
    const size = this.estimateEntrySize(data);
    this.cache.set(frameIndex, { ...data, size });
  }

  /**
   * Check if frame is cached
   */
  has(frameIndex: number): boolean {
    return this.cache.has(frameIndex);
  }

  /**
   * Clear cache
   */
  clear(): void {
    this.cache.clear();
  }

  /**
   * Get cache statistics
   */
  getStats(): { size: number; memoryUsage: number } {
    return {
      size: this.cache.size,
      memoryUsage: this.cache.memoryUsage,
    };
  }

  /**
   * Estimate size of frame data entry
   */
  private estimateEntrySize(data: Omit<FrameDataEntry, 'size'>): number {
    let size = 1000; // Base overhead

    if (data.decodedFrame) {
      // Estimate decoded frame size (width * height * 4 bytes for RGBA)
      const { width, height } = data;
      size += width * height * 4;
    }

    if (data.thumbnailData) {
      size += data.thumbnailData.length * 0.75; // Base64 encoding overhead
    }

    if (data.analysisData) {
      size += 500; // Estimate for analysis data
    }

    return Math.ceil(size);
  }
}

/**
 * Frame data entry
 */
export interface FrameDataEntry {
  frameIndex: number;
  width: number;
  height: number;
  decodedFrame?: string; // Data URL
  thumbnailData?: string;
  thumbnailUrl?: string;
  analysisData?: AnalysisData;
  size: number; // Estimated size in bytes
}

/**
 * Analysis data interface
 */
export interface AnalysisData {
  qpGrid?: QPGrid;
  mvGrid?: MVGrid;
  partitionGrid?: PartitionGrid;
  [key: string]: unknown; // Allow other analysis types
}

/**
 * QP Grid data
 */
export interface QPGrid {
  width: number;
  height: number;
  qpValues: number[];
}

/**
 * MV Grid data
 */
export interface MVGrid {
  width: number;
  height: number;
  mvX: number[][];
  mvY: number[][];
}

/**
 * Partition Grid data
 */
export interface PartitionGrid {
  width: number;
  height: number;
  partitions: Partition[];
}

/**
 * Partition data
 */
export interface Partition {
  x: number;
  y: number;
  width: number;
  height: number;
  type: string;
}

/**
 * Create a frame data cache
 */
export function createFrameDataCache(maxFrames?: number, maxMemoryMB?: number): FrameDataCache {
  return new FrameDataCache(maxFrames, maxMemoryMB);
}
