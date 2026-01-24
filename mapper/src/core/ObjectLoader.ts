import type { ObjectDefinition, ObjectsConfig } from '@/types';

export class ObjectLoader {
  private objects: Map<number, ObjectDefinition> = new Map();
  private walls: Map<number, ObjectDefinition> = new Map();
  private basePath: string = '';
  private wallsBasePath: string = '';
  private firstGid: number = 1001;
  private wallsFirstGid: number = 1;

  async loadObjects(config: ObjectsConfig): Promise<void> {
    this.basePath = config.basePath;
    this.firstGid = config.firstGid;

    const loadPromises = config.items.map(async (item) => {
      const obj: ObjectDefinition = {
        ...item,
        image: undefined,
      };

      // Load the image
      const image = new Image();
      const loadPromise = new Promise<void>((resolve) => {
        image.onload = () => {
          obj.image = image;
          resolve();
        };
        image.onerror = () => {
          console.warn(`Failed to load object image: ${item.id}.png`);
          resolve();
        };
      });

      image.src = `${this.basePath}/${item.id}.png`;
      await loadPromise;

      this.objects.set(item.id, obj);
    });

    await Promise.all(loadPromises);
  }

  getObject(id: number): ObjectDefinition | undefined {
    return this.objects.get(id);
  }

  getAllObjects(): ObjectDefinition[] {
    return Array.from(this.objects.values());
  }

  getObjectsWithImages(): ObjectDefinition[] {
    return this.getAllObjects().filter((obj) => obj.image !== undefined);
  }

  // Convert file ID to GID (for storage in chunk)
  idToGid(id: number): number {
    return this.firstGid + id;
  }

  // Convert GID back to file ID
  gidToId(gid: number): number {
    return gid - this.firstGid;
  }

  getFirstGid(): number {
    return this.firstGid;
  }

  getBasePath(): string {
    return this.basePath;
  }

  // Load wall sprites (separate from objects)
  async loadWalls(config: ObjectsConfig): Promise<void> {
    this.wallsBasePath = config.basePath;
    this.wallsFirstGid = config.firstGid;

    const loadPromises = config.items.map(async (item) => {
      const obj: ObjectDefinition = {
        ...item,
        image: undefined,
      };

      // Load the image
      const image = new Image();
      const loadPromise = new Promise<void>((resolve) => {
        image.onload = () => {
          obj.image = image;
          resolve();
        };
        image.onerror = () => {
          console.warn(`Failed to load wall image: ${item.id}.png`);
          resolve();
        };
      });

      image.src = `${this.wallsBasePath}/${item.id}.png`;
      await loadPromise;

      this.walls.set(item.id, obj);
    });

    await Promise.all(loadPromises);
  }

  getWall(id: number): ObjectDefinition | undefined {
    return this.walls.get(id);
  }

  getAllWalls(): ObjectDefinition[] {
    return Array.from(this.walls.values());
  }

  getWallsWithImages(): ObjectDefinition[] {
    return this.getAllWalls().filter((obj) => obj.image !== undefined);
  }

  // Convert wall file ID to GID
  wallIdToGid(id: number): number {
    return this.wallsFirstGid + id;
  }

  // Convert wall GID back to file ID
  wallGidToId(gid: number): number {
    return gid - this.wallsFirstGid;
  }

  // Get wall by GID
  getWallByGid(gid: number): ObjectDefinition | undefined {
    const id = this.wallGidToId(gid);
    return this.walls.get(id);
  }

  // Get object OR wall by GID (checks both)
  getObjectByGid(gid: number): ObjectDefinition | undefined {
    // Check walls first (wall GIDs are typically lower)
    if (gid >= this.wallsFirstGid && gid < this.wallsFirstGid + 10000) {
      const wall = this.getWallByGid(gid);
      if (wall) return wall;
    }
    // Fall back to objects
    const id = this.gidToId(gid);
    return this.objects.get(id);
  }

  getWallsFirstGid(): number {
    return this.wallsFirstGid;
  }

  getWallsBasePath(): string {
    return this.wallsBasePath;
  }
}

// Singleton instance
export const objectLoader = new ObjectLoader();
