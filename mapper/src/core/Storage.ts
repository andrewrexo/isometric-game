import type { Chunk, ChunkCoord } from '@/types';
import { chunkKey } from './coords';

const DB_NAME = 'mapper-storage';
const DB_VERSION = 1;
const CHUNKS_STORE = 'chunks';

// API base URL - empty string means same origin
const API_BASE = '';

class StorageManager {
  private db: IDBDatabase | null = null;
  private initPromise: Promise<void> | null = null;
  private _isConnected: boolean = false;
  private connectionListeners: Set<(connected: boolean) => void> = new Set();

  get isConnected(): boolean {
    return this._isConnected;
  }

  private setConnected(connected: boolean) {
    if (this._isConnected !== connected) {
      this._isConnected = connected;
      this.connectionListeners.forEach(listener => listener(connected));
    }
  }

  onConnectionChange(listener: (connected: boolean) => void): () => void {
    this.connectionListeners.add(listener);
    return () => this.connectionListeners.delete(listener);
  }

  // --- IndexedDB (local fallback) ---

  async initIndexedDB(): Promise<void> {
    if (this.db) return;
    if (this.initPromise) return this.initPromise;

    this.initPromise = new Promise((resolve, reject) => {
      const request = indexedDB.open(DB_NAME, DB_VERSION);

      request.onerror = () => {
        console.error('Failed to open IndexedDB:', request.error);
        reject(request.error);
      };

      request.onsuccess = () => {
        this.db = request.result;
        resolve();
      };

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains(CHUNKS_STORE)) {
          db.createObjectStore(CHUNKS_STORE, { keyPath: 'key' });
        }
      };
    });

    return this.initPromise;
  }

  async saveChunkLocal(chunk: Chunk): Promise<void> {
    await this.initIndexedDB();
    if (!this.db) return;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readwrite');
      const store = transaction.objectStore(CHUNKS_STORE);

      const storableChunk = {
        key: chunkKey(chunk.coord),
        ...chunk,
        collision: Array.from(chunk.collision),
      };

      const request = store.put(storableChunk);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  }

  async saveAllChunksLocal(chunks: Map<string, Chunk>): Promise<void> {
    await this.initIndexedDB();
    if (!this.db) return;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readwrite');
      const store = transaction.objectStore(CHUNKS_STORE);

      for (const [key, chunk] of chunks) {
        const storableChunk = {
          key,
          ...chunk,
          collision: Array.from(chunk.collision),
        };
        store.put(storableChunk);
      }

      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }

  async loadAllChunksLocal(): Promise<Map<string, Chunk>> {
    await this.initIndexedDB();
    if (!this.db) return new Map();

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readonly');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.getAll();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const chunks = new Map<string, Chunk>();

        for (const stored of request.result) {
          const chunk: Chunk = {
            coord: stored.coord,
            width: stored.width,
            height: stored.height,
            layers: stored.layers,
            collision: new Uint8Array(stored.collision),
            entities: stored.entities,
            mapObjects: stored.mapObjects || [],
            walls: stored.walls || [],
            dirty: stored.dirty,
          };
          chunks.set(stored.key, chunk);
        }

        resolve(chunks);
      };
    });
  }

  async hasLocalData(): Promise<boolean> {
    await this.initIndexedDB();
    if (!this.db) return false;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readonly');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.count();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result > 0);
    });
  }

  async clearLocal(): Promise<void> {
    await this.initIndexedDB();
    if (!this.db) return;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readwrite');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.clear();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  }

  // --- Server API ---

  private chunkToStorable(chunk: Chunk): object {
    return {
      ...chunk,
      collision: Array.from(chunk.collision),
    };
  }

  private storableToChunk(stored: Record<string, unknown>): Chunk {
    return {
      coord: stored.coord as ChunkCoord,
      width: stored.width as number,
      height: stored.height as number,
      layers: stored.layers as Chunk['layers'],
      collision: new Uint8Array(stored.collision as number[]),
      entities: stored.entities as Chunk['entities'],
      mapObjects: (stored.mapObjects as Chunk['mapObjects']) || [],
      walls: (stored.walls as Chunk['walls']) || [],
      dirty: false,
    };
  }

  async loadAllChunksFromServer(): Promise<Map<string, Chunk> | null> {
    try {
      const response = await fetch(`${API_BASE}/api/chunks/all`);
      if (!response.ok) {
        throw new Error(`Server responded with ${response.status}`);
      }

      const data = await response.json();
      const chunks = new Map<string, Chunk>();

      for (const [key, stored] of Object.entries(data)) {
        chunks.set(key, this.storableToChunk(stored as Record<string, unknown>));
      }

      this.setConnected(true);
      return chunks;
    } catch (err) {
      console.error('Failed to load from server:', err);
      this.setConnected(false);
      return null;
    }
  }

  async saveChunkToServer(chunk: Chunk): Promise<boolean> {
    try {
      const { cx, cy } = chunk.coord;
      const response = await fetch(`${API_BASE}/api/chunks/${cx}/${cy}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(this.chunkToStorable(chunk)),
      });

      if (!response.ok) {
        throw new Error(`Server responded with ${response.status}`);
      }

      this.setConnected(true);
      return true;
    } catch (err) {
      console.error('Failed to save chunk to server:', err);
      this.setConnected(false);
      return false;
    }
  }

  async saveAllChunksToServer(chunks: Map<string, Chunk>): Promise<boolean> {
    try {
      const payload: Record<string, object> = {};
      for (const [key, chunk] of chunks) {
        payload[key] = this.chunkToStorable(chunk);
      }

      const response = await fetch(`${API_BASE}/api/chunks`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (!response.ok) {
        throw new Error(`Server responded with ${response.status}`);
      }

      this.setConnected(true);
      return true;
    } catch (err) {
      console.error('Failed to save chunks to server:', err);
      this.setConnected(false);
      return false;
    }
  }

  async deleteChunkFromServer(coord: ChunkCoord): Promise<boolean> {
    try {
      const response = await fetch(`${API_BASE}/api/chunks/${coord.cx}/${coord.cy}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new Error(`Server responded with ${response.status}`);
      }

      this.setConnected(true);
      return true;
    } catch (err) {
      console.error('Failed to delete chunk from server:', err);
      this.setConnected(false);
      return false;
    }
  }

  // --- Combined Operations (Server + Local Fallback) ---

  async loadAllChunks(): Promise<Map<string, Chunk>> {
    // Try server first
    const serverChunks = await this.loadAllChunksFromServer();
    if (serverChunks !== null) {
      // Also save to local for offline backup
      await this.saveAllChunksLocal(serverChunks);
      return serverChunks;
    }

    // Fall back to local
    console.warn('Server unavailable, loading from local storage');
    return this.loadAllChunksLocal();
  }

  async saveAllChunks(chunks: Map<string, Chunk>): Promise<void> {
    // Always save locally
    await this.saveAllChunksLocal(chunks);

    // Try to save to server
    await this.saveAllChunksToServer(chunks);
  }

  async saveChunk(chunk: Chunk): Promise<void> {
    // Always save locally
    await this.saveChunkLocal(chunk);

    // Try to save to server
    await this.saveChunkToServer(chunk);
  }

  // --- Export/Import ---

  async exportMapData(): Promise<string> {
    const chunks = await this.loadAllChunksLocal();
    const payload: Record<string, object> = {};

    for (const [key, chunk] of chunks) {
      payload[key] = this.chunkToStorable(chunk);
    }

    return JSON.stringify({
      version: 1,
      exportedAt: new Date().toISOString(),
      chunks: payload,
    }, null, 2);
  }

  async importMapData(jsonString: string): Promise<number> {
    const data = JSON.parse(jsonString);
    const { chunks } = data;

    if (!chunks || typeof chunks !== 'object') {
      throw new Error('Invalid import format');
    }

    const chunkMap = new Map<string, Chunk>();
    for (const [key, stored] of Object.entries(chunks)) {
      chunkMap.set(key, this.storableToChunk(stored as Record<string, unknown>));
    }

    // Clear local and save new data
    await this.clearLocal();
    await this.saveAllChunksLocal(chunkMap);

    // Try to sync to server
    await this.saveAllChunksToServer(chunkMap);

    return chunkMap.size;
  }

  async importToServer(jsonString: string): Promise<number> {
    const data = JSON.parse(jsonString);

    const response = await fetch(`${API_BASE}/api/map/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      throw new Error(`Server responded with ${response.status}`);
    }

    const result = await response.json();
    return result.imported;
  }
}

export const storage = new StorageManager();
