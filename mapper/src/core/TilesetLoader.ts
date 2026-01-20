import type { TilesetConfig, Tileset, TileRect, MapperConfig } from '@/types';

export class TilesetLoader {
  private tilesets: Map<string, Tileset> = new Map();
  private gidToTileset: Map<number, Tileset> = new Map();
  private config: MapperConfig | null = null;

  async loadConfig(configPath: string = '/mapper-config.json'): Promise<MapperConfig> {
    const response = await fetch(configPath);
    if (!response.ok) {
      throw new Error(`Failed to load config: ${response.statusText}`);
    }
    this.config = await response.json();
    return this.config!;
  }

  async loadTilesets(configs: TilesetConfig[]): Promise<void> {
    const loadPromises = configs.map((config) => this.loadTileset(config));
    await Promise.all(loadPromises);
    this.buildGidMap();
  }

  private async loadTileset(config: TilesetConfig): Promise<Tileset> {
    const image = new Image();

    const loadPromise = new Promise<HTMLImageElement>((resolve, reject) => {
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error(`Failed to load tileset image: ${config.image}`));
    });

    image.src = config.image;
    await loadPromise;

    const rows = Math.floor(image.height / config.tileHeight);
    const tileCount = config.columns * rows;

    const tileset: Tileset = {
      ...config,
      imageElement: image,
      rows,
      tileCount,
    };

    this.tilesets.set(config.name, tileset);
    return tileset;
  }

  private buildGidMap(): void {
    this.gidToTileset.clear();

    // Sort tilesets by firstGid for proper lookup
    const sortedTilesets = Array.from(this.tilesets.values()).sort(
      (a, b) => a.firstGid - b.firstGid
    );

    for (const tileset of sortedTilesets) {
      for (let i = 0; i < tileset.tileCount; i++) {
        this.gidToTileset.set(tileset.firstGid + i, tileset);
      }
    }
  }

  getTileset(name: string): Tileset | undefined {
    return this.tilesets.get(name);
  }

  getTilesetForGid(gid: number): Tileset | undefined {
    if (gid <= 0) return undefined;

    // Find the tileset that contains this GID
    let result: Tileset | undefined;
    for (const tileset of this.tilesets.values()) {
      if (gid >= tileset.firstGid && gid < tileset.firstGid + tileset.tileCount) {
        result = tileset;
        break;
      }
    }
    return result;
  }

  // Get the source rectangle for a tile by its global ID
  getTileRect(gid: number): TileRect | null {
    if (gid <= 0) return null;

    const tileset = this.getTilesetForGid(gid);
    if (!tileset) return null;

    const localId = gid - tileset.firstGid;
    const col = localId % tileset.columns;
    const row = Math.floor(localId / tileset.columns);

    return {
      x: col * tileset.tileWidth,
      y: row * tileset.tileHeight,
      width: tileset.tileWidth,
      height: tileset.tileHeight,
    };
  }

  // Draw a tile onto a canvas context
  drawTile(
    ctx: CanvasRenderingContext2D,
    gid: number,
    destX: number,
    destY: number,
    scale: number = 1
  ): void {
    if (gid <= 0) return;

    const tileset = this.getTilesetForGid(gid);
    if (!tileset || !tileset.imageElement) return;

    const rect = this.getTileRect(gid);
    if (!rect) return;

    ctx.drawImage(
      tileset.imageElement,
      rect.x,
      rect.y,
      rect.width,
      rect.height,
      destX,
      destY,
      rect.width * scale,
      rect.height * scale
    );
  }

  getAllTilesets(): Tileset[] {
    return Array.from(this.tilesets.values());
  }

  getTileCount(): number {
    let total = 0;
    for (const tileset of this.tilesets.values()) {
      total += tileset.tileCount;
    }
    return total;
  }

  getConfig(): MapperConfig | null {
    return this.config;
  }
}

// Singleton instance
export const tilesetLoader = new TilesetLoader();
