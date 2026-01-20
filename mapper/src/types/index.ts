// Coordinate types
export interface ChunkCoord {
  cx: number;
  cy: number;
}

export interface LocalCoord {
  lx: number;
  ly: number;
}

export interface WorldCoord {
  wx: number;
  wy: number;
}

export interface ScreenCoord {
  sx: number;
  sy: number;
}

// Tileset types
export interface TilesetConfig {
  name: string;
  image: string;
  tileWidth: number;
  tileHeight: number;
  columns: number;
  firstGid: number;
}

export interface Tileset extends TilesetConfig {
  imageElement: HTMLImageElement | null;
  rows: number;
  tileCount: number;
}

export interface TileRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

// Layer types
export const Layer = {
  Ground: 'ground',
  Objects: 'objects',
  Overhead: 'overhead',
  Collision: 'collision',
  Entities: 'entities',
  MapObjects: 'mapObjects', // Trees, rocks, decorations
} as const;
export type Layer = typeof Layer[keyof typeof Layer];

// Tool types
export const Tool = {
  Select: 'select',
  Paint: 'paint',
  Fill: 'fill',
  Eraser: 'eraser',
  Collision: 'collision',
  Entity: 'entity',
  Object: 'object',
  Eyedropper: 'eyedropper',
} as const;
export type Tool = typeof Tool[keyof typeof Tool];

// Entity types
export interface EntitySpawn {
  id: string;
  entityId: string;
  name: string;
  x: number; // local x within chunk
  y: number; // local y within chunk
  level: number;
  uniqueId?: string;
  facing?: string;
  respawn?: boolean;
}

export interface EntityDefinition {
  id: string;
  displayName: string;
  sprite: string;
  description: string;
  behaviors: {
    hostile?: boolean;
    questGiver?: boolean;
    merchant?: boolean;
    craftsman?: boolean;
  };
}

export interface EntityRegistry {
  entities: Map<string, EntityDefinition>;
  byType: {
    hostile: EntityDefinition[];
    questGiver: EntityDefinition[];
    merchant: EntityDefinition[];
    other: EntityDefinition[];
  };
}

// Map object types (trees, rocks, decorations with cartesian coords)
export interface MapObject {
  id: string;
  gid: number; // Global tile ID from tileset
  x: number; // Local tile X within chunk
  y: number; // Local tile Y within chunk
  width: number; // Sprite width in pixels
  height: number; // Sprite height in pixels
}

// Chunk types
export interface ChunkLayer {
  name: string;
  data: number[];
}

export interface Chunk {
  coord: ChunkCoord;
  width: number;
  height: number;
  layers: {
    ground: number[];
    objects: number[];
    overhead: number[];
  };
  collision: Uint8Array; // Bitset for collision
  entities: EntitySpawn[];
  mapObjects: MapObject[]; // Trees, rocks, decorations
  dirty: boolean;
}

export interface World {
  chunks: Map<string, Chunk>;
  bounds: {
    minCx: number;
    maxCx: number;
    minCy: number;
    maxCy: number;
  };
}

// Object definition (for placeable map objects like trees, rocks)
export interface ObjectDefinition {
  id: number; // File ID (e.g., 101 for 101.png)
  name: string;
  width: number;
  height: number;
  image?: HTMLImageElement; // Loaded image
}

export interface ObjectsConfig {
  basePath: string;
  firstGid: number;
  items: Omit<ObjectDefinition, 'image'>[];
}

// Map config
export interface MapperConfig {
  tilesets: TilesetConfig[];
  objects?: ObjectsConfig;
  chunkSize: number;
  mapsPath: string;
  entitiesPath: string;
}

// Tiled format types (for import)
export interface TiledProperty {
  name: string;
  type: string;
  value: string | number | boolean;
}

export interface TiledObject {
  id: number;
  name: string;
  type: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
  visible: boolean;
  properties?: TiledProperty[];
}

export interface TiledLayer {
  id: number;
  name: string;
  type: 'tilelayer' | 'objectgroup';
  data?: number[];
  objects?: TiledObject[];
  width?: number;
  height?: number;
  visible: boolean;
  opacity: number;
  x: number;
  y: number;
}

export interface TiledTileset {
  firstgid: number;
  source: string;
}

export interface TiledMap {
  width: number;
  height: number;
  tilewidth: number;
  tileheight: number;
  layers: TiledLayer[];
  tilesets: TiledTileset[];
  orientation: string;
  type: string;
  version: string;
}

// New simplified format (for export)
export interface SimplifiedChunk {
  version: 2;
  coord: ChunkCoord;
  size: number;
  layers: {
    ground: number[];
    objects: number[];
    overhead: number[];
  };
  collision: string; // Base64 encoded bitset
  entities: SimplifiedEntitySpawn[];
  mapObjects: SimplifiedMapObject[];
}

export interface SimplifiedEntitySpawn {
  entityId: string;
  x: number;
  y: number;
  level: number;
  uniqueId?: string;
  facing?: string;
  respawn?: boolean;
}

export interface SimplifiedMapObject {
  gid: number;
  x: number; // Local tile X
  y: number; // Local tile Y
  width: number;
  height: number;
}

// History types
export interface HistoryAction {
  type: string;
  undo: () => void;
  redo: () => void;
}

// Selection types
export interface Selection {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

export interface Stamp {
  name: string;
  width: number;
  height: number;
  layers: {
    ground: number[];
    objects: number[];
    overhead: number[];
  };
  collision: boolean[];
  entities: EntitySpawn[];
}

// Viewport types
export interface Viewport {
  offsetX: number;
  offsetY: number;
  zoom: number;
}
