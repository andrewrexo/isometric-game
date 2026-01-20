import type {
  Chunk,
  ChunkCoord,
  World,
  TiledMap,
  TiledObject,
  EntitySpawn,
  MapObject,
  SimplifiedChunk,
  SimplifiedEntitySpawn,
  SimplifiedMapObject,
} from '@/types';
import { chunkKey, CHUNK_SIZE } from './coords';
import { BitSet } from './BitSet';

export class ChunkManager {
  private world: World = {
    chunks: new Map(),
    bounds: {
      minCx: 0,
      maxCx: 0,
      minCy: 0,
      maxCy: 0,
    },
  };

  // Load a chunk from JSON (auto-detects format)
  async loadChunk(path: string, coord: ChunkCoord): Promise<Chunk | null> {
    try {
      const response = await fetch(path);
      if (!response.ok) return null;

      const data = await response.json();

      // Detect format by checking for version field
      if ('version' in data && data.version >= 2) {
        return this.parseSimplifiedFormat(data as SimplifiedChunk, coord);
      } else {
        return this.parseTiledFormat(data as TiledMap, coord);
      }
    } catch (error) {
      console.warn(`Failed to load chunk at ${path}:`, error);
      return null;
    }
  }

  // Parse Tiled JSON format (current format)
  private parseTiledFormat(data: TiledMap, coord: ChunkCoord): Chunk {
    const width = data.width;
    const height = data.height;
    const tileSize = width * height;

    const chunk: Chunk = {
      coord,
      width,
      height,
      layers: {
        ground: new Array(tileSize).fill(0),
        objects: new Array(tileSize).fill(0),
        overhead: new Array(tileSize).fill(0),
      },
      collision: new Uint8Array(Math.ceil(tileSize / 8)),
      entities: [],
      mapObjects: [],
      dirty: false,
    };

    // Process layers
    for (const layer of data.layers) {
      if (layer.type === 'tilelayer' && layer.data) {
        const layerName = layer.name.toLowerCase();
        if (layerName === 'ground' || layerName === 'floor') {
          chunk.layers.ground = [...layer.data];
        } else if (layerName === 'objects' || layerName === 'object') {
          chunk.layers.objects = [...layer.data];
        } else if (layerName === 'overhead' || layerName === 'roof' || layerName === 'above') {
          chunk.layers.overhead = [...layer.data];
        } else if (layerName === 'collision') {
          // Convert tile data to collision bitset
          const bitset = new BitSet(tileSize);
          for (let i = 0; i < layer.data.length; i++) {
            if (layer.data[i] > 0) {
              bitset.set(i, true);
            }
          }
          chunk.collision = bitset.getRaw();
        }
      } else if (layer.type === 'objectgroup' && layer.objects) {
        // Parse entity spawns and map objects
        const { entities, mapObjects } = this.parseTiledObjects(layer.objects, data.tilewidth, data.tileheight);
        chunk.entities = entities;
        chunk.mapObjects = mapObjects;
      }
    }

    return chunk;
  }

  // Parse objects from Tiled format (entities and map objects)
  private parseTiledObjects(
    objects: TiledObject[],
    tileWidth: number,
    tileHeight: number
  ): { entities: EntitySpawn[]; mapObjects: MapObject[] } {
    const entities: EntitySpawn[] = [];
    const mapObjects: MapObject[] = [];

    for (const obj of objects) {
      // Check if object has a gid (it's a map object like tree/rock)
      if ('gid' in obj && typeof (obj as TiledObject & { gid?: number }).gid === 'number') {
        const gid = (obj as TiledObject & { gid: number }).gid;
        // Convert pixel coords to tile coords (Tiled uses tileHeight for isometric)
        const x = Math.floor(obj.x / 32); // Use 32 for isometric tile height
        const y = Math.floor(obj.y / 32);

        mapObjects.push({
          id: `obj_${obj.id}`,
          gid,
          x,
          y,
          width: obj.width,
          height: obj.height,
        });
        continue;
      }

      // Entity spawn
      if (obj.type !== 'entity_spawn') continue;

      // Convert pixel coordinates to tile coordinates
      const x = Math.floor(obj.x / tileWidth);
      const y = Math.floor(obj.y / tileHeight);

      // Extract properties
      let entityId = '';
      let level = 1;
      let uniqueId: string | undefined;
      let facing: string | undefined;
      let respawn: boolean | undefined;

      if (obj.properties) {
        for (const prop of obj.properties) {
          switch (prop.name) {
            case 'entity_id':
              entityId = String(prop.value);
              break;
            case 'level':
              level = Number(prop.value);
              break;
            case 'unique_id':
              uniqueId = String(prop.value);
              break;
            case 'facing':
              facing = String(prop.value);
              break;
            case 'respawn':
              respawn = Boolean(prop.value);
              break;
          }
        }
      }

      if (entityId) {
        entities.push({
          id: `entity_${obj.id}`,
          entityId,
          name: obj.name,
          x,
          y,
          level,
          uniqueId,
          facing,
          respawn,
        });
      }
    }

    return { entities, mapObjects };
  }

  // Parse new simplified format
  private parseSimplifiedFormat(data: SimplifiedChunk, coord: ChunkCoord): Chunk {
    const size = data.size;
    const tileSize = size * size;

    // Decode collision from base64
    const collisionBitset = BitSet.fromBase64(data.collision, tileSize);

    const chunk: Chunk = {
      coord,
      width: size,
      height: size,
      layers: {
        ground: [...data.layers.ground],
        objects: [...data.layers.objects],
        overhead: [...data.layers.overhead],
      },
      collision: collisionBitset.getRaw(),
      entities: data.entities.map((e, i) => ({
        id: `entity_${i}`,
        entityId: e.entityId,
        name: e.entityId,
        x: e.x,
        y: e.y,
        level: e.level,
        uniqueId: e.uniqueId,
        facing: e.facing,
        respawn: e.respawn,
      })),
      mapObjects: (data.mapObjects || []).map((o, i) => ({
        id: `obj_${i}`,
        gid: o.gid,
        x: o.x,
        y: o.y,
        width: o.width,
        height: o.height,
      })),
      dirty: false,
    };

    return chunk;
  }

  // Load multiple chunks from a directory
  async loadChunksFromDirectory(basePath: string, chunkCoords: ChunkCoord[]): Promise<void> {
    const loadPromises = chunkCoords.map(async (coord) => {
      const path = `${basePath}/chunk_${coord.cx}_${coord.cy}.json`;
      const chunk = await this.loadChunk(path, coord);
      if (chunk) {
        this.addChunk(chunk);
      }
    });

    await Promise.all(loadPromises);
    this.updateBounds();
  }

  // Scan for available chunks (requires server-side file listing)
  async discoverChunks(basePath: string): Promise<ChunkCoord[]> {
    // In browser, we need an index file or scan a known range
    // For now, scan a reasonable range around origin
    const coords: ChunkCoord[] = [];
    const range = 5; // Check -5 to +5 in both directions

    for (let cx = -range; cx <= range; cx++) {
      for (let cy = -range; cy <= range; cy++) {
        coords.push({ cx, cy });
      }
    }

    // Try to load each chunk to see if it exists
    const existingCoords: ChunkCoord[] = [];
    for (const coord of coords) {
      const path = `${basePath}/chunk_${coord.cx}_${coord.cy}.json`;
      try {
        const response = await fetch(path, { method: 'HEAD' });
        if (response.ok) {
          existingCoords.push(coord);
        }
      } catch {
        // Chunk doesn't exist
      }
    }

    return existingCoords;
  }

  addChunk(chunk: Chunk): void {
    const key = chunkKey(chunk.coord);
    this.world.chunks.set(key, chunk);
    this.updateBounds();
  }

  getChunk(coord: ChunkCoord): Chunk | undefined {
    return this.world.chunks.get(chunkKey(coord));
  }

  getChunkByKey(key: string): Chunk | undefined {
    return this.world.chunks.get(key);
  }

  getAllChunks(): Chunk[] {
    return Array.from(this.world.chunks.values());
  }

  getDirtyChunks(): Chunk[] {
    return this.getAllChunks().filter((c) => c.dirty);
  }

  markChunkDirty(coord: ChunkCoord): void {
    const chunk = this.getChunk(coord);
    if (chunk) {
      chunk.dirty = true;
    }
  }

  markChunkClean(coord: ChunkCoord): void {
    const chunk = this.getChunk(coord);
    if (chunk) {
      chunk.dirty = false;
    }
  }

  private updateBounds(): void {
    if (this.world.chunks.size === 0) {
      this.world.bounds = { minCx: 0, maxCx: 0, minCy: 0, maxCy: 0 };
      return;
    }

    let minCx = Infinity,
      maxCx = -Infinity;
    let minCy = Infinity,
      maxCy = -Infinity;

    for (const chunk of this.world.chunks.values()) {
      minCx = Math.min(minCx, chunk.coord.cx);
      maxCx = Math.max(maxCx, chunk.coord.cx);
      minCy = Math.min(minCy, chunk.coord.cy);
      maxCy = Math.max(maxCy, chunk.coord.cy);
    }

    this.world.bounds = { minCx, maxCx, minCy, maxCy };
  }

  getWorld(): World {
    return this.world;
  }

  getBounds(): World['bounds'] {
    return this.world.bounds;
  }

  // Export chunk to simplified format
  exportChunk(coord: ChunkCoord): SimplifiedChunk | null {
    const chunk = this.getChunk(coord);
    if (!chunk) return null;
    return this.exportChunkData(chunk);
  }

  // Export a chunk object directly (for exporting from store)
  exportChunkData(chunk: Chunk): SimplifiedChunk {
    const collisionBitset = new BitSet(chunk.width * chunk.height);
    collisionBitset.setRaw(chunk.collision);

    const entities: SimplifiedEntitySpawn[] = chunk.entities.map((e) => ({
      entityId: e.entityId,
      x: e.x,
      y: e.y,
      level: e.level,
      uniqueId: e.uniqueId,
      facing: e.facing,
      respawn: e.respawn,
    }));

    const mapObjects: SimplifiedMapObject[] = chunk.mapObjects.map((o) => ({
      gid: o.gid,
      x: o.x,
      y: o.y,
      width: o.width,
      height: o.height,
    }));

    return {
      version: 2,
      coord: chunk.coord,
      size: chunk.width,
      layers: {
        ground: [...chunk.layers.ground],
        objects: [...chunk.layers.objects],
        overhead: [...chunk.layers.overhead],
      },
      collision: collisionBitset.toBase64(),
      entities,
      mapObjects,
    };
  }

  // Export chunk to JSON string
  exportChunkToJSON(coord: ChunkCoord, pretty: boolean = true): string | null {
    const data = this.exportChunk(coord);
    if (!data) return null;
    return pretty ? JSON.stringify(data, null, 2) : JSON.stringify(data);
  }

  // Export a chunk object directly to JSON string
  exportChunkDataToJSON(chunk: Chunk, pretty: boolean = true): string {
    const data = this.exportChunkData(chunk);
    return pretty ? JSON.stringify(data, null, 2) : JSON.stringify(data);
  }

  // Create a new empty chunk
  createEmptyChunk(coord: ChunkCoord): Chunk {
    const tileSize = CHUNK_SIZE * CHUNK_SIZE;

    const chunk: Chunk = {
      coord,
      width: CHUNK_SIZE,
      height: CHUNK_SIZE,
      layers: {
        ground: new Array(tileSize).fill(0),
        objects: new Array(tileSize).fill(0),
        overhead: new Array(tileSize).fill(0),
      },
      collision: new Uint8Array(Math.ceil(tileSize / 8)),
      entities: [],
      mapObjects: [],
      dirty: true,
    };

    this.addChunk(chunk);
    return chunk;
  }

  // Get or create chunk at coordinate
  getOrCreateChunk(coord: ChunkCoord): Chunk {
    const existing = this.getChunk(coord);
    if (existing) return existing;
    return this.createEmptyChunk(coord);
  }

  // Clear all chunks
  clear(): void {
    this.world.chunks.clear();
    this.world.bounds = { minCx: 0, maxCx: 0, minCy: 0, maxCy: 0 };
  }
}

// Singleton instance
export const chunkManager = new ChunkManager();
