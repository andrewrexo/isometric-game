import type { ObjectDefinition, ObjectsConfig } from '@/types';

export class ObjectLoader {
  private objects: Map<number, ObjectDefinition> = new Map();
  private basePath: string = '';
  private firstGid: number = 1001;

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

  // Get object by GID
  getObjectByGid(gid: number): ObjectDefinition | undefined {
    const id = this.gidToId(gid);
    return this.objects.get(id);
  }

  getFirstGid(): number {
    return this.firstGid;
  }

  getBasePath(): string {
    return this.basePath;
  }
}

// Singleton instance
export const objectLoader = new ObjectLoader();
