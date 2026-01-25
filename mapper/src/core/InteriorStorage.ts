import type { InteriorMap, SerializedInteriorMap, SpawnPoint, ExitPortal } from '@/types';
import { BitSet } from './BitSet';

const API_BASE = '';

class InteriorStorageManager {
  private interiors: Map<string, InteriorMap> = new Map();
  private interiorIds: string[] = [];

  // Get list of available interior map IDs
  getInteriorIds(): string[] {
    return [...this.interiorIds];
  }

  // Get a loaded interior map
  getInterior(id: string): InteriorMap | undefined {
    return this.interiors.get(id);
  }

  // Check if an interior is loaded
  hasInterior(id: string): boolean {
    return this.interiors.has(id);
  }

  // Create a new interior map
  createInterior(id: string, name: string, width: number = 16, height: number = 16): InteriorMap {
    const tileCount = width * height;

    const interior: InteriorMap = {
      id,
      name,
      instanceType: 'public',
      width,
      height,
      spawnPoints: [{ name: 'entrance', x: Math.floor(width / 2), y: height - 1 }],
      layers: {
        ground: new Array(tileCount).fill(0),
        objects: new Array(tileCount).fill(0),
        overhead: new Array(tileCount).fill(0),
      },
      collision: new Uint8Array(Math.ceil(tileCount / 8)),
      entities: [],
      mapObjects: [],
      walls: [],
      exitPortals: [],
      dirty: true,
    };

    this.interiors.set(id, interior);
    if (!this.interiorIds.includes(id)) {
      this.interiorIds.push(id);
      this.interiorIds.sort();
    }

    return interior;
  }

  // Load interior from serialized format
  private deserializeInterior(data: SerializedInteriorMap): InteriorMap {
    const { width, height } = data.size;
    const tileCount = width * height;

    // Convert spawn_points object to array
    const spawnPoints: SpawnPoint[] = Object.entries(data.spawn_points).map(([name, pos]) => ({
      name,
      x: pos.x,
      y: pos.y,
    }));

    // Decode collision
    const collisionBitset = BitSet.fromBase64(data.collision, tileCount);

    // Convert exit portals
    const exitPortals: ExitPortal[] = (data.portals || []).map((p, i) => ({
      id: p.id || `exit_${i}`,
      x: p.x,
      y: p.y,
      width: p.width || 1,
      height: p.height || 1,
      targetX: p.target_x,
      targetY: p.target_y,
    }));

    return {
      id: data.id,
      name: data.name,
      instanceType: data.instance_type,
      width,
      height,
      spawnPoints,
      layers: {
        ground: [...data.layers.ground],
        objects: [...data.layers.objects],
        overhead: [...data.layers.overhead],
      },
      collision: collisionBitset.getRaw(),
      entities: (data.entities || []).map((e, i) => ({
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
      walls: (data.walls || []).map((w, i) => ({
        id: `wall_${i}`,
        gid: w.gid,
        x: w.x,
        y: w.y,
        edge: w.edge,
      })),
      exitPortals,
      dirty: false,
    };
  }

  // Serialize interior for saving/exporting
  serializeInterior(interior: InteriorMap): SerializedInteriorMap {
    const collisionBitset = new BitSet(interior.width * interior.height);
    collisionBitset.setRaw(interior.collision);

    // Convert spawn points array to object
    const spawnPointsObj: { [name: string]: { x: number; y: number } } = {};
    for (const sp of interior.spawnPoints) {
      spawnPointsObj[sp.name] = { x: sp.x, y: sp.y };
    }

    return {
      id: interior.id,
      name: interior.name,
      instance_type: interior.instanceType,
      size: { width: interior.width, height: interior.height },
      spawn_points: spawnPointsObj,
      layers: {
        ground: [...interior.layers.ground],
        objects: [...interior.layers.objects],
        overhead: [...interior.layers.overhead],
      },
      collision: collisionBitset.toBase64(),
      entities: interior.entities.map((e) => ({
        entityId: e.entityId,
        x: e.x,
        y: e.y,
        level: e.level,
        uniqueId: e.uniqueId,
        facing: e.facing,
        respawn: e.respawn,
      })),
      mapObjects: interior.mapObjects.map((o) => ({
        gid: o.gid,
        x: o.x,
        y: o.y,
        width: o.width,
        height: o.height,
      })),
      walls: interior.walls.map((w) => ({
        gid: w.gid,
        x: w.x,
        y: w.y,
        edge: w.edge,
      })),
      portals: interior.exitPortals.map((p) => ({
        id: p.id,
        x: p.x,
        y: p.y,
        width: p.width,
        height: p.height,
        target_map: 'overworld',
        target_x: p.targetX,
        target_y: p.targetY,
      })),
    };
  }

  // Load list of available interiors from server
  async loadInteriorList(): Promise<string[]> {
    try {
      const response = await fetch(`${API_BASE}/api/interiors`);
      if (!response.ok) {
        console.warn('Failed to load interior list from server');
        return [];
      }
      const data = await response.json();
      this.interiorIds = data.interiors || [];
      return this.interiorIds;
    } catch (err) {
      console.warn('Failed to load interior list:', err);
      return [];
    }
  }

  // Load a specific interior from server
  async loadInterior(id: string): Promise<InteriorMap | null> {
    // Check if already loaded
    if (this.interiors.has(id)) {
      return this.interiors.get(id)!;
    }

    try {
      const response = await fetch(`${API_BASE}/api/interiors/${id}`);
      if (!response.ok) {
        console.warn(`Failed to load interior ${id}`);
        return null;
      }
      const data: SerializedInteriorMap = await response.json();
      const interior = this.deserializeInterior(data);
      this.interiors.set(id, interior);

      if (!this.interiorIds.includes(id)) {
        this.interiorIds.push(id);
        this.interiorIds.sort();
      }

      return interior;
    } catch (err) {
      console.warn(`Failed to load interior ${id}:`, err);
      return null;
    }
  }

  // Save interior to server
  async saveInterior(interior: InteriorMap): Promise<boolean> {
    try {
      const data = this.serializeInterior(interior);
      const response = await fetch(`${API_BASE}/api/interiors/${interior.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });

      if (!response.ok) {
        console.error(`Failed to save interior ${interior.id}`);
        return false;
      }

      // Update the local cache with the saved interior
      this.interiors.set(interior.id, { ...interior, dirty: false });

      interior.dirty = false;
      return true;
    } catch (err) {
      console.error(`Failed to save interior ${interior.id}:`, err);
      return false;
    }
  }

  // Delete interior from server
  async deleteInterior(id: string): Promise<boolean> {
    try {
      const response = await fetch(`${API_BASE}/api/interiors/${id}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        console.error(`Failed to delete interior ${id}`);
        return false;
      }

      this.interiors.delete(id);
      this.interiorIds = this.interiorIds.filter((i) => i !== id);
      return true;
    } catch (err) {
      console.error(`Failed to delete interior ${id}:`, err);
      return false;
    }
  }

  // Update a loaded interior (or add it to the cache if not present)
  updateInterior(id: string, updates: Partial<InteriorMap>): void {
    const interior = this.interiors.get(id);
    if (interior) {
      Object.assign(interior, updates, { dirty: true });
    } else if ('id' in updates && 'width' in updates && 'height' in updates) {
      // Full interior object passed - add to cache
      this.interiors.set(id, updates as InteriorMap);
      if (!this.interiorIds.includes(id)) {
        this.interiorIds.push(id);
        this.interiorIds.sort();
      }
    }
  }

  // Get spawn points for an interior
  getSpawnPoints(id: string): SpawnPoint[] {
    const interior = this.interiors.get(id);
    return interior?.spawnPoints || [];
  }

  // Clear all loaded interiors
  clear(): void {
    this.interiors.clear();
  }

  // Get all loaded interiors (for exporting)
  getAllInteriors(): InteriorMap[] {
    return Array.from(this.interiors.values());
  }

  // Export an interior as JSON string
  exportInteriorToJSON(interior: InteriorMap): string {
    const data = this.serializeInterior(interior);
    return JSON.stringify(data, null, 2);
  }
}

export const interiorStorage = new InteriorStorageManager();
