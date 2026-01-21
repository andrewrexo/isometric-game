import { create } from 'zustand';
import type {
  Chunk,
  ChunkCoord,
  WorldCoord,
  Viewport,
  EntitySpawn,
  EntityRegistry,
  Tileset,
  Selection,
  MapObject,
} from '@/types';
import { Tool, Layer } from '@/types';
import { chunkKey, worldToChunk, worldToLocal, localToIndex } from '@/core/coords';
import { BitSet } from '@/core/BitSet';
import { history } from '@/core/History';
import { chunkManager } from '@/core/ChunkManager';
import { objectLoader } from '@/core/ObjectLoader';
import { storage } from '@/core/Storage';

// Debounce helper for auto-save
let saveTimeout: ReturnType<typeof setTimeout> | null = null;
const debouncedSave = (chunks: Map<string, Chunk>) => {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(() => {
    storage.saveAllChunks(chunks).catch((err) => {
      console.error('Failed to auto-save chunks:', err);
    });
  }, 500); // Save 500ms after last change
};

interface EditorState {
  // World state
  chunks: Map<string, Chunk>;
  worldBounds: {
    minCx: number;
    maxCx: number;
    minCy: number;
    maxCy: number;
  };

  // View state
  viewport: Viewport;
  hoveredTile: WorldCoord | null;
  selection: Selection | null;

  // Tool state
  activeTool: Tool;
  activeLayer: Layer;
  selectedTileId: number;
  selectedEntityId: string | null; // Entity type ID from palette (for placing new entities)
  selectedObjectId: number | null; // File ID of selected object for placement

  // Selection state (for placed items on map)
  selectedEntitySpawn: { chunkCoord: ChunkCoord; spawnId: string } | null;
  selectedMapObject: { chunkCoord: ChunkCoord; objectId: string } | null;

  // Magic wand tile selection
  selectedTiles: Set<string>; // Set of "wx,wy" coordinate strings

  // Asset state
  tilesets: Tileset[];
  entityRegistry: EntityRegistry | null;

  // UI state
  showGrid: boolean;
  showChunkBounds: boolean;
  showCollision: boolean;
  showEntities: boolean;
  showMapObjects: boolean;
  visibleLayers: {
    ground: boolean;
    objects: boolean;
    overhead: boolean;
  };

  // Loading state
  isLoading: boolean;
  loadingMessage: string;

  // Connection state
  isConnected: boolean;
}

interface EditorActions {
  // World actions
  setChunks: (chunks: Map<string, Chunk>, skipAutoSave?: boolean) => void;
  updateChunk: (coord: ChunkCoord, updater: (chunk: Chunk) => Chunk) => void;
  setWorldBounds: (bounds: EditorState['worldBounds']) => void;

  // View actions
  setViewport: (viewport: Partial<Viewport>) => void;
  pan: (dx: number, dy: number) => void;
  zoom: (factor: number, centerX: number, centerY: number) => void;
  setHoveredTile: (tile: WorldCoord | null) => void;
  setSelection: (selection: Selection | null) => void;

  // Tool actions
  setActiveTool: (tool: Tool) => void;
  setActiveLayer: (layer: Layer) => void;
  setSelectedTileId: (id: number) => void;
  setSelectedEntityId: (id: string | null) => void;
  setSelectedObjectId: (id: number | null) => void;

  // Selection actions (for placed items)
  setSelectedEntitySpawn: (selection: { chunkCoord: ChunkCoord; spawnId: string } | null) => void;
  setSelectedMapObject: (selection: { chunkCoord: ChunkCoord; objectId: string } | null) => void;
  findEntityAtWorld: (world: WorldCoord) => { chunkCoord: ChunkCoord; entity: EntitySpawn } | null;
  findMapObjectAtWorld: (world: WorldCoord) => { chunkCoord: ChunkCoord; object: MapObject } | null;

  // Tile editing actions
  setTile: (world: WorldCoord, layer: Layer, tileId: number) => void;
  toggleCollision: (world: WorldCoord) => void;
  fillTiles: (start: WorldCoord, layer: Layer, tileId: number) => void;

  // Magic wand selection
  magicWandSelect: (world: WorldCoord, layer: Layer) => void;
  clearSelectedTiles: () => void;
  fillSelectedTiles: (layer: Layer, tileId: number) => void;

  // Entity actions
  addEntity: (world: WorldCoord, entityId: string) => EntitySpawn;
  removeEntity: (chunkCoord: ChunkCoord, entitySpawnId: string) => void;
  updateEntity: (chunkCoord: ChunkCoord, entitySpawnId: string, updates: Partial<EntitySpawn>) => void;

  // Map object actions
  addMapObject: (world: WorldCoord, objectId: number, width: number, height: number) => MapObject;
  removeMapObject: (chunkCoord: ChunkCoord, objectSpawnId: string) => void;
  updateMapObject: (chunkCoord: ChunkCoord, objectSpawnId: string, updates: Partial<MapObject>) => void;

  // Asset actions
  setTilesets: (tilesets: Tileset[]) => void;
  setEntityRegistry: (registry: EntityRegistry) => void;

  // UI actions
  toggleGrid: () => void;
  toggleChunkBounds: () => void;
  toggleCollisionOverlay: () => void;
  toggleEntitiesOverlay: () => void;
  toggleMapObjectsOverlay: () => void;
  setLayerVisibility: (layer: keyof EditorState['visibleLayers'], visible: boolean) => void;

  // History actions
  undo: () => void;
  redo: () => void;

  // Loading actions
  setLoading: (isLoading: boolean, message?: string) => void;

  // Connection actions
  setConnected: (isConnected: boolean) => void;

  // Utility
  getChunk: (coord: ChunkCoord) => Chunk | undefined;
  getOrCreateChunk: (coord: ChunkCoord) => Chunk;
  getDirtyChunks: () => Chunk[];
  markAllClean: () => void;
}

export const useEditorStore = create<EditorState & EditorActions>((set, get) => ({
  // Initial state
  chunks: new Map(),
  worldBounds: { minCx: 0, maxCx: 0, minCy: 0, maxCy: 0 },

  viewport: {
    offsetX: 400,
    offsetY: 200,
    zoom: 1,
  },
  hoveredTile: null,
  selection: null,

  activeTool: Tool.Paint,
  activeLayer: Layer.Ground,
  selectedTileId: 1,
  selectedEntityId: null,
  selectedObjectId: null,

  selectedEntitySpawn: null,
  selectedMapObject: null,
  selectedTiles: new Set(),

  tilesets: [],
  entityRegistry: null,

  showGrid: false,
  showChunkBounds: true,
  showCollision: false,
  showEntities: true,
  showMapObjects: true,
  visibleLayers: {
    ground: true,
    objects: true,
    overhead: true,
  },

  isLoading: false,
  loadingMessage: '',

  isConnected: false,

  // World actions
  setChunks: (chunks, skipAutoSave = false) => {
    const newChunks = new Map(chunks);
    set({ chunks: newChunks });
    if (!skipAutoSave) {
      debouncedSave(newChunks);
    }
  },

  updateChunk: (coord, updater) => {
    const chunks = new Map(get().chunks);
    const key = chunkKey(coord);
    const chunk = chunks.get(key);
    if (chunk) {
      chunks.set(key, updater(chunk));
      set({ chunks });
      debouncedSave(chunks);
    }
  },

  setWorldBounds: (bounds) => set({ worldBounds: bounds }),

  // View actions
  setViewport: (viewport) => {
    set((state) => ({
      viewport: { ...state.viewport, ...viewport },
    }));
  },

  pan: (dx, dy) => {
    set((state) => ({
      viewport: {
        ...state.viewport,
        offsetX: state.viewport.offsetX + dx,
        offsetY: state.viewport.offsetY + dy,
      },
    }));
  },

  zoom: (factor, centerX, centerY) => {
    set((state) => {
      const newZoom = Math.max(0.25, Math.min(4, state.viewport.zoom * factor));
      const zoomRatio = newZoom / state.viewport.zoom;

      // Adjust offset to zoom toward center point
      const newOffsetX = centerX - (centerX - state.viewport.offsetX) * zoomRatio;
      const newOffsetY = centerY - (centerY - state.viewport.offsetY) * zoomRatio;

      return {
        viewport: {
          offsetX: newOffsetX,
          offsetY: newOffsetY,
          zoom: newZoom,
        },
      };
    });
  },

  setHoveredTile: (tile) => set({ hoveredTile: tile }),
  setSelection: (selection) => set({ selection }),

  // Tool actions
  setActiveTool: (tool) => set({ activeTool: tool }),
  setActiveLayer: (layer) => set({ activeLayer: layer }),
  setSelectedTileId: (id) => set({ selectedTileId: id }),
  setSelectedEntityId: (id) => set({ selectedEntityId: id }),
  setSelectedObjectId: (id) => set({ selectedObjectId: id }),

  // Selection actions (for placed items)
  setSelectedEntitySpawn: (selection) => set({ selectedEntitySpawn: selection }),
  setSelectedMapObject: (selection) => set({ selectedMapObject: selection }),

  findEntityAtWorld: (world) => {
    const chunkCoord = worldToChunk(world);
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return null;

    const local = worldToLocal(world);
    // Find entity at this position
    const entity = chunk.entities.find((e) => e.x === local.lx && e.y === local.ly);
    if (entity) {
      return { chunkCoord, entity };
    }
    return null;
  },

  findMapObjectAtWorld: (world) => {
    const chunkCoord = worldToChunk(world);
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return null;

    const local = worldToLocal(world);
    // Find object at this position
    const object = chunk.mapObjects.find((o) => o.x === local.lx && o.y === local.ly);
    if (object) {
      return { chunkCoord, object };
    }
    return null;
  },

  // Tile editing actions
  setTile: (world, layer, tileId) => {
    const chunkCoord = worldToChunk(world);
    const chunk = get().getOrCreateChunk(chunkCoord);
    const local = worldToLocal(world);
    const index = localToIndex(local);

    const layerKey = layer === Layer.Ground ? 'ground' : layer === Layer.Objects ? 'objects' : 'overhead';
    if (layerKey === 'ground' || layerKey === 'objects' || layerKey === 'overhead') {
      const oldTileId = chunk.layers[layerKey][index];
      if (oldTileId === tileId) return; // No change

      // Record for undo
      history.push({
        type: 'setTile',
        description: `Set tile at ${world.wx},${world.wy}`,
        undo: () => get().setTile(world, layer, oldTileId),
        redo: () => get().setTile(world, layer, tileId),
      });

      get().updateChunk(chunkCoord, (c) => ({
        ...c,
        layers: {
          ...c.layers,
          [layerKey]: c.layers[layerKey].map((t, i) => (i === index ? tileId : t)),
        },
        dirty: true,
      }));
    }
  },

  toggleCollision: (world) => {
    const chunkCoord = worldToChunk(world);
    const chunk = get().getOrCreateChunk(chunkCoord);
    const local = worldToLocal(world);
    const index = localToIndex(local);

    const bitset = new BitSet(chunk.width * chunk.height);
    bitset.setRaw(chunk.collision);
    const oldValue = bitset.get(index);
    const newValue = !oldValue;

    history.push({
      type: 'toggleCollision',
      description: `Toggle collision at ${world.wx},${world.wy}`,
      undo: () => {
        const c = get().getChunk(chunkCoord);
        if (c) {
          const bs = new BitSet(c.width * c.height);
          bs.setRaw(c.collision);
          bs.set(index, oldValue);
          get().updateChunk(chunkCoord, (ch) => ({
            ...ch,
            collision: bs.getRaw(),
            dirty: true,
          }));
        }
      },
      redo: () => {
        const c = get().getChunk(chunkCoord);
        if (c) {
          const bs = new BitSet(c.width * c.height);
          bs.setRaw(c.collision);
          bs.set(index, newValue);
          get().updateChunk(chunkCoord, (ch) => ({
            ...ch,
            collision: bs.getRaw(),
            dirty: true,
          }));
        }
      },
    });

    bitset.set(index, newValue);
    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      collision: bitset.getRaw(),
      dirty: true,
    }));
  },

  fillTiles: (start, layer, tileId) => {
    const chunkCoord = worldToChunk(start);
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return;

    const layerKey = layer === Layer.Ground ? 'ground' : layer === Layer.Objects ? 'objects' : 'overhead';
    if (layerKey !== 'ground' && layerKey !== 'objects' && layerKey !== 'overhead') return;

    const local = worldToLocal(start);
    const startIndex = localToIndex(local);
    const targetTileId = chunk.layers[layerKey][startIndex];

    if (targetTileId === tileId) return; // Same tile, no fill needed

    // Flood fill algorithm
    const filled = new Set<number>();
    const toFill: number[] = [startIndex];
    const changes: { index: number; oldTile: number }[] = [];

    while (toFill.length > 0) {
      const index = toFill.pop()!;
      if (filled.has(index)) continue;
      if (index < 0 || index >= chunk.width * chunk.height) continue;

      const currentTile = chunk.layers[layerKey][index];
      if (currentTile !== targetTileId) continue;

      filled.add(index);
      changes.push({ index, oldTile: currentTile });

      const x = index % chunk.width;
      const y = Math.floor(index / chunk.width);

      // Add neighbors
      if (x > 0) toFill.push(index - 1);
      if (x < chunk.width - 1) toFill.push(index + 1);
      if (y > 0) toFill.push(index - chunk.width);
      if (y < chunk.height - 1) toFill.push(index + chunk.width);
    }

    if (changes.length === 0) return;

    // Record for undo
    history.push({
      type: 'fillTiles',
      description: `Fill ${changes.length} tiles`,
      undo: () => {
        get().updateChunk(chunkCoord, (c) => {
          const newLayer = [...c.layers[layerKey]];
          for (const change of changes) {
            newLayer[change.index] = change.oldTile;
          }
          return {
            ...c,
            layers: { ...c.layers, [layerKey]: newLayer },
            dirty: true,
          };
        });
      },
      redo: () => {
        get().updateChunk(chunkCoord, (c) => {
          const newLayer = [...c.layers[layerKey]];
          for (const change of changes) {
            newLayer[change.index] = tileId;
          }
          return {
            ...c,
            layers: { ...c.layers, [layerKey]: newLayer },
            dirty: true,
          };
        });
      },
    });

    // Apply fill
    get().updateChunk(chunkCoord, (c) => {
      const newLayer = [...c.layers[layerKey]];
      for (const change of changes) {
        newLayer[change.index] = tileId;
      }
      return {
        ...c,
        layers: { ...c.layers, [layerKey]: newLayer },
        dirty: true,
      };
    });
  },

  // Magic wand selection - select ALL tiles of the same type in the chunk
  magicWandSelect: (start, layer) => {
    const chunkCoord = worldToChunk(start);
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return;

    const layerKey = layer === Layer.Ground ? 'ground' : layer === Layer.Objects ? 'objects' : 'overhead';
    if (layerKey !== 'ground' && layerKey !== 'objects' && layerKey !== 'overhead') return;

    const local = worldToLocal(start);
    const startIndex = localToIndex(local);
    const targetTileId = chunk.layers[layerKey][startIndex];

    // Find ALL tiles with the same ID in the chunk
    const selectedTiles = new Set<string>();
    const baseX = chunkCoord.cx * chunk.width;
    const baseY = chunkCoord.cy * chunk.height;

    for (let index = 0; index < chunk.width * chunk.height; index++) {
      if (chunk.layers[layerKey][index] === targetTileId) {
        const lx = index % chunk.width;
        const ly = Math.floor(index / chunk.width);
        const wx = baseX + lx;
        const wy = baseY + ly;
        selectedTiles.add(`${wx},${wy}`);
      }
    }

    set({ selectedTiles });
  },

  clearSelectedTiles: () => {
    set({ selectedTiles: new Set() });
  },

  fillSelectedTiles: (layer, tileId) => {
    const { selectedTiles } = get();
    if (selectedTiles.size === 0) return;

    const layerKey = layer === Layer.Ground ? 'ground' : layer === Layer.Objects ? 'objects' : 'overhead';
    if (layerKey !== 'ground' && layerKey !== 'objects' && layerKey !== 'overhead') return;

    // Group tiles by chunk for efficient updates
    const tilesByChunk = new Map<string, { wx: number; wy: number }[]>();

    for (const tileKey of selectedTiles) {
      const [wxStr, wyStr] = tileKey.split(',');
      const wx = parseInt(wxStr, 10);
      const wy = parseInt(wyStr, 10);
      const chunkCoord = worldToChunk({ wx, wy });
      const key = chunkKey(chunkCoord);

      if (!tilesByChunk.has(key)) {
        tilesByChunk.set(key, []);
      }
      tilesByChunk.get(key)!.push({ wx, wy });
    }

    // Record changes for undo
    const changes: { chunkCoord: ChunkCoord; index: number; oldTile: number }[] = [];

    for (const [key, tiles] of tilesByChunk) {
      const chunk = get().chunks.get(key);
      if (!chunk) continue;

      for (const { wx, wy } of tiles) {
        const local = worldToLocal({ wx, wy });
        const index = localToIndex(local);
        const oldTile = chunk.layers[layerKey][index];
        if (oldTile !== tileId) {
          changes.push({ chunkCoord: chunk.coord, index, oldTile });
        }
      }
    }

    if (changes.length === 0) {
      set({ selectedTiles: new Set() });
      return;
    }

    // Record for undo
    history.push({
      type: 'fillSelectedTiles',
      description: `Fill ${changes.length} selected tiles`,
      undo: () => {
        // Restore old tiles
        const changesByChunk = new Map<string, { index: number; oldTile: number }[]>();
        for (const change of changes) {
          const key = chunkKey(change.chunkCoord);
          if (!changesByChunk.has(key)) {
            changesByChunk.set(key, []);
          }
          changesByChunk.get(key)!.push({ index: change.index, oldTile: change.oldTile });
        }

        for (const [cKey, chunkChanges] of changesByChunk) {
          const coords = cKey.split(',').map(Number);
          get().updateChunk({ cx: coords[0], cy: coords[1] }, (c) => {
            const newLayer = [...c.layers[layerKey]];
            for (const { index, oldTile } of chunkChanges) {
              newLayer[index] = oldTile;
            }
            return {
              ...c,
              layers: { ...c.layers, [layerKey]: newLayer },
              dirty: true,
            };
          });
        }
      },
      redo: () => {
        get().fillSelectedTiles(layer, tileId);
      },
    });

    // Apply fill to all chunks
    for (const [key, tiles] of tilesByChunk) {
      const chunk = get().chunks.get(key);
      if (!chunk) continue;

      get().updateChunk(chunk.coord, (c) => {
        const newLayer = [...c.layers[layerKey]];
        for (const { wx, wy } of tiles) {
          const local = worldToLocal({ wx, wy });
          const index = localToIndex(local);
          newLayer[index] = tileId;
        }
        return {
          ...c,
          layers: { ...c.layers, [layerKey]: newLayer },
          dirty: true,
        };
      });
    }

    // Clear selection after fill
    set({ selectedTiles: new Set() });
  },

  // Entity actions
  addEntity: (world, entityId) => {
    const chunkCoord = worldToChunk(world);
    get().getOrCreateChunk(chunkCoord); // Ensure chunk exists
    const local = worldToLocal(world);

    const newEntity: EntitySpawn = {
      id: `entity_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      entityId,
      name: entityId,
      x: local.lx,
      y: local.ly,
      level: 1,
    };

    history.push({
      type: 'addEntity',
      description: `Add entity ${entityId}`,
      undo: () => get().removeEntity(chunkCoord, newEntity.id),
      redo: () => {
        get().updateChunk(chunkCoord, (c) => ({
          ...c,
          entities: [...c.entities, newEntity],
          dirty: true,
        }));
      },
    });

    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      entities: [...c.entities, newEntity],
      dirty: true,
    }));

    return newEntity;
  },

  removeEntity: (chunkCoord, entitySpawnId) => {
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return;

    const entity = chunk.entities.find((e) => e.id === entitySpawnId);
    if (!entity) return;

    history.push({
      type: 'removeEntity',
      description: `Remove entity ${entity.entityId}`,
      undo: () => {
        get().updateChunk(chunkCoord, (c) => ({
          ...c,
          entities: [...c.entities, entity],
          dirty: true,
        }));
      },
      redo: () => get().removeEntity(chunkCoord, entitySpawnId),
    });

    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      entities: c.entities.filter((e) => e.id !== entitySpawnId),
      dirty: true,
    }));
  },

  updateEntity: (chunkCoord, entitySpawnId, updates) => {
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return;

    const entityIndex = chunk.entities.findIndex((e) => e.id === entitySpawnId);
    if (entityIndex === -1) return;

    const oldEntity = { ...chunk.entities[entityIndex] };
    const newEntity = { ...oldEntity, ...updates };

    history.push({
      type: 'updateEntity',
      description: `Update entity ${oldEntity.entityId}`,
      undo: () => get().updateEntity(chunkCoord, entitySpawnId, oldEntity),
      redo: () => get().updateEntity(chunkCoord, entitySpawnId, updates),
    });

    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      entities: c.entities.map((e, i) => (i === entityIndex ? newEntity : e)),
      dirty: true,
    }));
  },

  // Map object actions
  addMapObject: (world, objectId, width, height) => {
    const chunkCoord = worldToChunk(world);
    get().getOrCreateChunk(chunkCoord); // Ensure chunk exists
    const local = worldToLocal(world);

    const gid = objectLoader.idToGid(objectId);
    const newObject: MapObject = {
      id: `obj_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      gid,
      x: local.lx,
      y: local.ly,
      width,
      height,
    };

    history.push({
      type: 'addMapObject',
      description: `Add map object ${objectId}`,
      undo: () => get().removeMapObject(chunkCoord, newObject.id),
      redo: () => {
        get().updateChunk(chunkCoord, (c) => ({
          ...c,
          mapObjects: [...c.mapObjects, newObject],
          dirty: true,
        }));
      },
    });

    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      mapObjects: [...c.mapObjects, newObject],
      dirty: true,
    }));

    return newObject;
  },

  removeMapObject: (chunkCoord, objectSpawnId) => {
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return;

    const obj = chunk.mapObjects.find((o) => o.id === objectSpawnId);
    if (!obj) return;

    history.push({
      type: 'removeMapObject',
      description: `Remove map object`,
      undo: () => {
        get().updateChunk(chunkCoord, (c) => ({
          ...c,
          mapObjects: [...c.mapObjects, obj],
          dirty: true,
        }));
      },
      redo: () => get().removeMapObject(chunkCoord, objectSpawnId),
    });

    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      mapObjects: c.mapObjects.filter((o) => o.id !== objectSpawnId),
      dirty: true,
    }));
  },

  updateMapObject: (chunkCoord, objectSpawnId, updates) => {
    const chunk = get().getChunk(chunkCoord);
    if (!chunk) return;

    const objectIndex = chunk.mapObjects.findIndex((o) => o.id === objectSpawnId);
    if (objectIndex === -1) return;

    const oldObject = { ...chunk.mapObjects[objectIndex] };
    const newObject = { ...oldObject, ...updates };

    history.push({
      type: 'updateMapObject',
      description: `Update map object`,
      undo: () => get().updateMapObject(chunkCoord, objectSpawnId, oldObject),
      redo: () => get().updateMapObject(chunkCoord, objectSpawnId, updates),
    });

    get().updateChunk(chunkCoord, (c) => ({
      ...c,
      mapObjects: c.mapObjects.map((o, i) => (i === objectIndex ? newObject : o)),
      dirty: true,
    }));
  },

  // Asset actions
  setTilesets: (tilesets) => set({ tilesets }),
  setEntityRegistry: (registry) => set({ entityRegistry: registry }),

  // UI actions
  toggleGrid: () => set((state) => ({ showGrid: !state.showGrid })),
  toggleChunkBounds: () => set((state) => ({ showChunkBounds: !state.showChunkBounds })),
  toggleCollisionOverlay: () => set((state) => ({ showCollision: !state.showCollision })),
  toggleEntitiesOverlay: () => set((state) => ({ showEntities: !state.showEntities })),
  toggleMapObjectsOverlay: () => set((state) => ({ showMapObjects: !state.showMapObjects })),
  setLayerVisibility: (layer, visible) =>
    set((state) => ({
      visibleLayers: { ...state.visibleLayers, [layer]: visible },
    })),

  // History actions
  undo: () => {
    history.undo();
  },
  redo: () => {
    history.redo();
  },

  // Loading actions
  setLoading: (isLoading, message = '') => set({ isLoading, loadingMessage: message }),

  // Connection actions
  setConnected: (isConnected) => set({ isConnected }),

  // Utility
  getChunk: (coord) => get().chunks.get(chunkKey(coord)),

  getOrCreateChunk: (coord) => {
    const existing = get().chunks.get(chunkKey(coord));
    if (existing) return existing;

    const newChunk = chunkManager.createEmptyChunk(coord);
    const chunks = new Map(get().chunks);
    chunks.set(chunkKey(coord), newChunk);
    set({ chunks });
    debouncedSave(chunks);
    return newChunk;
  },

  getDirtyChunks: () => {
    return Array.from(get().chunks.values()).filter((c) => c.dirty);
  },

  markAllClean: () => {
    const chunks = new Map(get().chunks);
    for (const [key, chunk] of chunks) {
      if (chunk.dirty) {
        chunks.set(key, { ...chunk, dirty: false });
      }
    }
    set({ chunks });
  },
}));
