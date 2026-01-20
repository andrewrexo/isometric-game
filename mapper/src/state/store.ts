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
} from '@/types';
import { Tool, Layer } from '@/types';
import { chunkKey, worldToChunk, worldToLocal, localToIndex } from '@/core/coords';
import { BitSet } from '@/core/BitSet';
import { history } from '@/core/History';
import { chunkManager } from '@/core/ChunkManager';

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
  selectedEntityId: string | null;

  // Asset state
  tilesets: Tileset[];
  entityRegistry: EntityRegistry | null;

  // UI state
  showGrid: boolean;
  showChunkBounds: boolean;
  showCollision: boolean;
  showEntities: boolean;
  visibleLayers: {
    ground: boolean;
    objects: boolean;
    overhead: boolean;
  };

  // Loading state
  isLoading: boolean;
  loadingMessage: string;
}

interface EditorActions {
  // World actions
  setChunks: (chunks: Map<string, Chunk>) => void;
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

  // Tile editing actions
  setTile: (world: WorldCoord, layer: Layer, tileId: number) => void;
  toggleCollision: (world: WorldCoord) => void;
  fillTiles: (start: WorldCoord, layer: Layer, tileId: number) => void;

  // Entity actions
  addEntity: (world: WorldCoord, entityId: string) => EntitySpawn;
  removeEntity: (chunkCoord: ChunkCoord, entitySpawnId: string) => void;
  updateEntity: (chunkCoord: ChunkCoord, entitySpawnId: string, updates: Partial<EntitySpawn>) => void;

  // Asset actions
  setTilesets: (tilesets: Tileset[]) => void;
  setEntityRegistry: (registry: EntityRegistry) => void;

  // UI actions
  toggleGrid: () => void;
  toggleChunkBounds: () => void;
  toggleCollisionOverlay: () => void;
  toggleEntitiesOverlay: () => void;
  setLayerVisibility: (layer: keyof EditorState['visibleLayers'], visible: boolean) => void;

  // History actions
  undo: () => void;
  redo: () => void;

  // Loading actions
  setLoading: (isLoading: boolean, message?: string) => void;

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

  tilesets: [],
  entityRegistry: null,

  showGrid: false,
  showChunkBounds: true,
  showCollision: false,
  showEntities: true,
  visibleLayers: {
    ground: true,
    objects: true,
    overhead: true,
  },

  isLoading: false,
  loadingMessage: '',

  // World actions
  setChunks: (chunks) => {
    set({ chunks: new Map(chunks) });
  },

  updateChunk: (coord, updater) => {
    const chunks = new Map(get().chunks);
    const key = chunkKey(coord);
    const chunk = chunks.get(key);
    if (chunk) {
      chunks.set(key, updater(chunk));
      set({ chunks });
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

  // Asset actions
  setTilesets: (tilesets) => set({ tilesets }),
  setEntityRegistry: (registry) => set({ entityRegistry: registry }),

  // UI actions
  toggleGrid: () => set((state) => ({ showGrid: !state.showGrid })),
  toggleChunkBounds: () => set((state) => ({ showChunkBounds: !state.showChunkBounds })),
  toggleCollisionOverlay: () => set((state) => ({ showCollision: !state.showCollision })),
  toggleEntitiesOverlay: () => set((state) => ({ showEntities: !state.showEntities })),
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

  // Utility
  getChunk: (coord) => get().chunks.get(chunkKey(coord)),

  getOrCreateChunk: (coord) => {
    const existing = get().chunks.get(chunkKey(coord));
    if (existing) return existing;

    const newChunk = chunkManager.createEmptyChunk(coord);
    const chunks = new Map(get().chunks);
    chunks.set(chunkKey(coord), newChunk);
    set({ chunks });
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
