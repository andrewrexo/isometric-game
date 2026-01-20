import type { Chunk, ChunkCoord } from '@/types';
import { chunkKey } from './coords';

const DB_NAME = 'mapper-storage';
const DB_VERSION = 1;
const CHUNKS_STORE = 'chunks';

class StorageManager {
  private db: IDBDatabase | null = null;
  private initPromise: Promise<void> | null = null;

  async init(): Promise<void> {
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

        // Create chunks store
        if (!db.objectStoreNames.contains(CHUNKS_STORE)) {
          db.createObjectStore(CHUNKS_STORE, { keyPath: 'key' });
        }
      };
    });

    return this.initPromise;
  }

  async saveChunk(chunk: Chunk): Promise<void> {
    await this.init();
    if (!this.db) return;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readwrite');
      const store = transaction.objectStore(CHUNKS_STORE);

      // Convert Uint8Array to regular array for storage
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

  async saveAllChunks(chunks: Map<string, Chunk>): Promise<void> {
    await this.init();
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

  async loadAllChunks(): Promise<Map<string, Chunk>> {
    await this.init();
    if (!this.db) return new Map();

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readonly');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.getAll();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const chunks = new Map<string, Chunk>();

        for (const stored of request.result) {
          // Convert array back to Uint8Array
          const chunk: Chunk = {
            coord: stored.coord,
            width: stored.width,
            height: stored.height,
            layers: stored.layers,
            collision: new Uint8Array(stored.collision),
            entities: stored.entities,
            mapObjects: stored.mapObjects,
            dirty: stored.dirty,
          };
          chunks.set(stored.key, chunk);
        }

        resolve(chunks);
      };
    });
  }

  async hasStoredData(): Promise<boolean> {
    await this.init();
    if (!this.db) return false;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readonly');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.count();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result > 0);
    });
  }

  async clearAll(): Promise<void> {
    await this.init();
    if (!this.db) return;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readwrite');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.clear();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  }

  async deleteChunk(coord: ChunkCoord): Promise<void> {
    await this.init();
    if (!this.db) return;

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([CHUNKS_STORE], 'readwrite');
      const store = transaction.objectStore(CHUNKS_STORE);
      const request = store.delete(chunkKey(coord));

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  }
}

export const storage = new StorageManager();
