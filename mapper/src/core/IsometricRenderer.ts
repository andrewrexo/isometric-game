import type { Chunk, Viewport, WorldCoord, MapObject, Wall, InteriorMap } from '@/types';
import {
  TILE_WIDTH,
  TILE_HEIGHT,
  CHUNK_SIZE,
  worldToScreen,
  chunkLocalToWorld,
  indexToLocal
} from './coords';
import { BitSet } from './BitSet';
import { tilesetLoader } from './TilesetLoader';
import { objectLoader } from './ObjectLoader';

export interface RenderOptions {
  showGrid: boolean;
  showChunkBounds: boolean;
  showCollision: boolean;
  showEntities: boolean;
  showMapObjects: boolean;
  showPortals: boolean;
  visibleLayers: {
    ground: boolean;
    objects: boolean;
    overhead: boolean;
  };
}

const DEFAULT_RENDER_OPTIONS: RenderOptions = {
  showGrid: false,
  showChunkBounds: true,
  showCollision: false,
  showEntities: true,
  showMapObjects: true,
  showPortals: true,
  visibleLayers: {
    ground: true,
    objects: true,
    overhead: true,
  },
};

export class IsometricRenderer {
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private options: RenderOptions = DEFAULT_RENDER_OPTIONS;

  attach(canvas: HTMLCanvasElement): void {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    if (this.ctx) {
      this.ctx.imageSmoothingEnabled = false;
    }
  }

  detach(): void {
    this.canvas = null;
    this.ctx = null;
  }

  setOptions(options: Partial<RenderOptions>): void {
    this.options = { ...this.options, ...options };
  }

  getOptions(): RenderOptions {
    return this.options;
  }

  clear(): void {
    if (!this.ctx || !this.canvas) return;
    this.ctx.fillStyle = '#1a1a2e';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
  }

  render(chunks: Chunk[], viewport: Viewport): void {
    if (!this.ctx || !this.canvas) return;

    this.clear();

    // Sort chunks for proper depth ordering (back to front)
    const sortedChunks = [...chunks].sort((a, b) => {
      const sumA = a.coord.cx + a.coord.cy;
      const sumB = b.coord.cx + b.coord.cy;
      return sumA - sumB;
    });

    // Render each chunk
    for (const chunk of sortedChunks) {
      this.renderChunk(chunk, viewport);
    }

    // Render overlays
    if (this.options.showChunkBounds) {
      for (const chunk of sortedChunks) {
        this.renderChunkBounds(chunk, viewport);
      }
    }

    if (this.options.showCollision) {
      for (const chunk of sortedChunks) {
        this.renderCollisionOverlay(chunk, viewport);
      }
    }

    // Render map objects and walls together (depth sorted)
    if (this.options.showMapObjects) {
      for (const chunk of sortedChunks) {
        this.renderMapObjectsAndWalls(chunk, viewport);
      }
    }

    if (this.options.showEntities) {
      for (const chunk of sortedChunks) {
        this.renderEntities(chunk, viewport);
      }
    }

    if (this.options.showPortals) {
      for (const chunk of sortedChunks) {
        this.renderPortals(chunk, viewport);
      }
    }

    if (this.options.showGrid) {
      this.renderGrid(viewport);
    }
  }

  // Render an interior map (fixed-size, not chunk-based)
  renderInterior(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx || !this.canvas) return;

    this.clear();

    // Render tiles
    for (let y = 0; y < interior.height; y++) {
      for (let x = 0; x < interior.width; x++) {
        const index = y * interior.width + x;
        const worldCoord: WorldCoord = { wx: x, wy: y };
        const screen = worldToScreen(worldCoord, viewport);

        const drawX = screen.sx - (TILE_WIDTH / 2) * viewport.zoom;
        const drawY = screen.sy;

        // Render ground layer
        if (this.options.visibleLayers.ground) {
          const groundTile = interior.layers.ground[index];
          if (groundTile > 0) {
            tilesetLoader.drawTile(this.ctx, groundTile, drawX, drawY, viewport.zoom);
          }
        }

        // Render objects layer
        if (this.options.visibleLayers.objects) {
          const objectTile = interior.layers.objects[index];
          if (objectTile > 0) {
            tilesetLoader.drawTile(this.ctx, objectTile, drawX, drawY, viewport.zoom);
          }
        }

        // Render overhead layer
        if (this.options.visibleLayers.overhead) {
          const overheadTile = interior.layers.overhead[index];
          if (overheadTile > 0) {
            tilesetLoader.drawTile(this.ctx, overheadTile, drawX, drawY, viewport.zoom);
          }
        }
      }
    }

    // Render interior bounds
    this.renderInteriorBounds(interior, viewport);

    // Render collision overlay
    if (this.options.showCollision) {
      this.renderInteriorCollision(interior, viewport);
    }

    // Render map objects and walls
    if (this.options.showMapObjects) {
      this.renderInteriorObjectsAndWalls(interior, viewport);
    }

    // Render entities
    if (this.options.showEntities) {
      this.renderInteriorEntities(interior, viewport);
    }

    // Render spawn points
    this.renderInteriorSpawnPoints(interior, viewport);

    // Render exit portals
    this.renderInteriorExitPortals(interior, viewport);

    if (this.options.showGrid) {
      this.renderGrid(viewport);
    }
  }

  private renderInteriorBounds(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx) return;

    const corners: WorldCoord[] = [
      { wx: 0, wy: 0 },
      { wx: interior.width, wy: 0 },
      { wx: interior.width, wy: interior.height },
      { wx: 0, wy: interior.height },
    ];

    const screenCorners = corners.map((c) => worldToScreen(c, viewport));

    this.ctx.strokeStyle = interior.dirty ? '#ff6b6b' : '#4ecdc4';
    this.ctx.lineWidth = 3;
    this.ctx.beginPath();
    this.ctx.moveTo(screenCorners[0].sx, screenCorners[0].sy);
    for (let i = 1; i < screenCorners.length; i++) {
      this.ctx.lineTo(screenCorners[i].sx, screenCorners[i].sy);
    }
    this.ctx.closePath();
    this.ctx.stroke();

    // Draw interior name label
    const centerWorld: WorldCoord = {
      wx: interior.width / 2,
      wy: interior.height / 2,
    };
    const centerScreen = worldToScreen(centerWorld, viewport);

    this.ctx.fillStyle = '#ffffff';
    this.ctx.font = `bold ${14 * viewport.zoom}px monospace`;
    this.ctx.textAlign = 'center';
    this.ctx.textBaseline = 'middle';
    this.ctx.fillText(interior.name, centerScreen.sx, centerScreen.sy - 20 * viewport.zoom);
  }

  private renderInteriorCollision(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx) return;

    const bitset = new BitSet(interior.width * interior.height);
    bitset.setRaw(interior.collision);

    this.ctx.fillStyle = 'rgba(255, 0, 0, 0.3)';

    for (let i = 0; i < interior.width * interior.height; i++) {
      if (bitset.get(i)) {
        const x = i % interior.width;
        const y = Math.floor(i / interior.width);
        const worldCoord: WorldCoord = { wx: x, wy: y };
        const screen = worldToScreen(worldCoord, viewport);

        const hw = (TILE_WIDTH / 2) * viewport.zoom;
        const hh = (TILE_HEIGHT / 2) * viewport.zoom;

        this.ctx.beginPath();
        this.ctx.moveTo(screen.sx, screen.sy);
        this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
        this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
        this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
        this.ctx.closePath();
        this.ctx.fill();
      }
    }
  }

  private renderInteriorObjectsAndWalls(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx) return;

    type Renderable =
      | { type: 'object'; obj: MapObject }
      | { type: 'wall'; wall: Wall };

    const renderables: Array<{ depth: number; item: Renderable }> = [];

    for (const obj of interior.mapObjects) {
      renderables.push({
        depth: obj.x + obj.y,
        item: { type: 'object', obj }
      });
    }

    for (const wall of interior.walls) {
      renderables.push({
        depth: wall.x + wall.y,
        item: { type: 'wall', wall }
      });
    }

    renderables.sort((a, b) => a.depth - b.depth);

    for (const { item } of renderables) {
      if (item.type === 'object') {
        this.renderInteriorMapObject(item.obj, viewport);
      } else {
        this.renderInteriorWall(item.wall, viewport);
      }
    }
  }

  private renderInteriorMapObject(obj: MapObject, viewport: Viewport): void {
    if (!this.ctx) return;

    const worldCoord: WorldCoord = { wx: obj.x, wy: obj.y };
    const screen = worldToScreen(worldCoord, viewport);

    const objDef = objectLoader.getObjectByGid(obj.gid);

    if (objDef?.image) {
      const scaledWidth = obj.width * viewport.zoom;
      const scaledHeight = obj.height * viewport.zoom;

      const drawX = screen.sx - scaledWidth / 2;
      const drawY = screen.sy + TILE_HEIGHT * viewport.zoom - scaledHeight;

      this.ctx.drawImage(
        objDef.image,
        0,
        0,
        objDef.image.width,
        objDef.image.height,
        drawX,
        drawY,
        scaledWidth,
        scaledHeight
      );
    }
  }

  private renderInteriorWall(wall: Wall, viewport: Viewport): void {
    if (!this.ctx) return;

    const worldCoord: WorldCoord = { wx: wall.x, wy: wall.y };
    const screen = worldToScreen(worldCoord, viewport);

    const objDef = objectLoader.getObjectByGid(wall.gid);

    if (objDef?.image) {
      const scaledWidth = objDef.image.width * viewport.zoom;
      const scaledHeight = objDef.image.height * viewport.zoom;

      const bottomVertexX = screen.sx;
      const bottomVertexY = screen.sy + TILE_HEIGHT * viewport.zoom;

      let drawX: number;
      let drawY: number;

      if (wall.edge === 'down') {
        drawX = bottomVertexX - scaledWidth;
        drawY = bottomVertexY - scaledHeight;
      } else {
        drawX = bottomVertexX;
        drawY = bottomVertexY - scaledHeight;
      }

      this.ctx.drawImage(
        objDef.image,
        0,
        0,
        objDef.image.width,
        objDef.image.height,
        drawX,
        drawY,
        scaledWidth,
        scaledHeight
      );
    }
  }

  private renderInteriorEntities(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx) return;

    for (const entity of interior.entities) {
      const worldCoord: WorldCoord = { wx: entity.x, wy: entity.y };
      const screen = worldToScreen(worldCoord, viewport);

      const size = 8 * viewport.zoom;
      this.ctx.fillStyle = '#ffd93d';
      this.ctx.strokeStyle = '#000000';
      this.ctx.lineWidth = 1;

      this.ctx.beginPath();
      this.ctx.arc(screen.sx, screen.sy + (TILE_HEIGHT / 2) * viewport.zoom, size, 0, Math.PI * 2);
      this.ctx.fill();
      this.ctx.stroke();

      if (viewport.zoom >= 0.5) {
        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = `${10 * Math.max(1, viewport.zoom)}px sans-serif`;
        this.ctx.textAlign = 'center';
        this.ctx.fillText(
          entity.name || entity.entityId,
          screen.sx,
          screen.sy + (TILE_HEIGHT / 2) * viewport.zoom - size - 4
        );
      }
    }
  }

  private renderInteriorSpawnPoints(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx) return;

    for (const spawn of interior.spawnPoints) {
      const worldCoord: WorldCoord = { wx: spawn.x, wy: spawn.y };
      const screen = worldToScreen(worldCoord, viewport);

      const hw = (TILE_WIDTH / 2) * viewport.zoom;
      const hh = (TILE_HEIGHT / 2) * viewport.zoom;

      // Draw green diamond for spawn point
      this.ctx.fillStyle = 'rgba(0, 255, 100, 0.5)';
      this.ctx.beginPath();
      this.ctx.moveTo(screen.sx, screen.sy);
      this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
      this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
      this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
      this.ctx.closePath();
      this.ctx.fill();

      this.ctx.strokeStyle = 'rgba(0, 255, 100, 0.9)';
      this.ctx.lineWidth = 2;
      this.ctx.stroke();

      // Draw spawn name
      if (viewport.zoom >= 0.5) {
        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = `bold ${10 * Math.max(1, viewport.zoom)}px sans-serif`;
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(
          spawn.name,
          screen.sx,
          screen.sy + (TILE_HEIGHT / 2) * viewport.zoom
        );
      }
    }
  }

  private renderInteriorExitPortals(interior: InteriorMap, viewport: Viewport): void {
    if (!this.ctx) return;

    for (const exitPortal of interior.exitPortals) {
      for (let py = 0; py < exitPortal.height; py++) {
        for (let px = 0; px < exitPortal.width; px++) {
          const worldCoord: WorldCoord = { wx: exitPortal.x + px, wy: exitPortal.y + py };
          const screen = worldToScreen(worldCoord, viewport);

          const hw = (TILE_WIDTH / 2) * viewport.zoom;
          const hh = (TILE_HEIGHT / 2) * viewport.zoom;

          // Draw orange diamond for exit portal
          this.ctx.fillStyle = 'rgba(255, 165, 0, 0.5)';
          this.ctx.beginPath();
          this.ctx.moveTo(screen.sx, screen.sy);
          this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
          this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
          this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
          this.ctx.closePath();
          this.ctx.fill();

          this.ctx.strokeStyle = 'rgba(255, 165, 0, 0.9)';
          this.ctx.lineWidth = 2;
          this.ctx.stroke();
        }
      }

      // Draw exit label
      if (viewport.zoom >= 0.5) {
        const worldCoord: WorldCoord = { wx: exitPortal.x, wy: exitPortal.y };
        const screen = worldToScreen(worldCoord, viewport);

        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = `bold ${10 * Math.max(1, viewport.zoom)}px sans-serif`;
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(
          'EXIT',
          screen.sx,
          screen.sy + (TILE_HEIGHT / 2) * viewport.zoom
        );
      }
    }
  }

  private renderChunk(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    // Render tiles in depth order (back to front for isometric)
    for (let y = 0; y < chunk.height; y++) {
      for (let x = 0; x < chunk.width; x++) {
        const index = y * chunk.width + x;
        const worldCoord = chunkLocalToWorld(chunk.coord, { lx: x, ly: y });
        const screen = worldToScreen(worldCoord, viewport);

        // Calculate draw position (top-left of tile)
        const drawX = screen.sx - (TILE_WIDTH / 2) * viewport.zoom;
        const drawY = screen.sy;

        // Render ground layer
        if (this.options.visibleLayers.ground) {
          const groundTile = chunk.layers.ground[index];
          if (groundTile > 0) {
            tilesetLoader.drawTile(this.ctx, groundTile, drawX, drawY, viewport.zoom);
          }
        }

        // Render objects layer
        if (this.options.visibleLayers.objects) {
          const objectTile = chunk.layers.objects[index];
          if (objectTile > 0) {
            tilesetLoader.drawTile(this.ctx, objectTile, drawX, drawY, viewport.zoom);
          }
        }

        // Render overhead layer
        if (this.options.visibleLayers.overhead) {
          const overheadTile = chunk.layers.overhead[index];
          if (overheadTile > 0) {
            tilesetLoader.drawTile(this.ctx, overheadTile, drawX, drawY, viewport.zoom);
          }
        }
      }
    }
  }

  private renderChunkBounds(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    const corners: WorldCoord[] = [
      { wx: chunk.coord.cx * CHUNK_SIZE, wy: chunk.coord.cy * CHUNK_SIZE },
      { wx: (chunk.coord.cx + 1) * CHUNK_SIZE, wy: chunk.coord.cy * CHUNK_SIZE },
      { wx: (chunk.coord.cx + 1) * CHUNK_SIZE, wy: (chunk.coord.cy + 1) * CHUNK_SIZE },
      { wx: chunk.coord.cx * CHUNK_SIZE, wy: (chunk.coord.cy + 1) * CHUNK_SIZE },
    ];

    const screenCorners = corners.map((c) => worldToScreen(c, viewport));

    this.ctx.strokeStyle = chunk.dirty ? '#ff6b6b' : '#4ecdc4';
    this.ctx.lineWidth = 2;
    this.ctx.beginPath();
    this.ctx.moveTo(screenCorners[0].sx, screenCorners[0].sy);
    for (let i = 1; i < screenCorners.length; i++) {
      this.ctx.lineTo(screenCorners[i].sx, screenCorners[i].sy);
    }
    this.ctx.closePath();
    this.ctx.stroke();

    // Draw chunk label
    const centerWorld: WorldCoord = {
      wx: chunk.coord.cx * CHUNK_SIZE + CHUNK_SIZE / 2,
      wy: chunk.coord.cy * CHUNK_SIZE + CHUNK_SIZE / 2,
    };
    const centerScreen = worldToScreen(centerWorld, viewport);

    this.ctx.fillStyle = '#ffffff';
    this.ctx.font = `${12 * viewport.zoom}px monospace`;
    this.ctx.textAlign = 'center';
    this.ctx.textBaseline = 'middle';
    this.ctx.fillText(`${chunk.coord.cx},${chunk.coord.cy}`, centerScreen.sx, centerScreen.sy);
  }

  private renderCollisionOverlay(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    const bitset = new BitSet(chunk.width * chunk.height);
    bitset.setRaw(chunk.collision);

    this.ctx.fillStyle = 'rgba(255, 0, 0, 0.3)';

    for (let i = 0; i < chunk.width * chunk.height; i++) {
      if (bitset.get(i)) {
        const local = indexToLocal(i, chunk.width);
        const worldCoord = chunkLocalToWorld(chunk.coord, local);
        const screen = worldToScreen(worldCoord, viewport);

        // Draw diamond shape for collision
        const hw = (TILE_WIDTH / 2) * viewport.zoom;
        const hh = (TILE_HEIGHT / 2) * viewport.zoom;

        this.ctx.beginPath();
        this.ctx.moveTo(screen.sx, screen.sy);
        this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
        this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
        this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
        this.ctx.closePath();
        this.ctx.fill();
      }
    }
  }

  private renderMapObjectsAndWalls(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    // Create unified renderables list for depth sorting
    type Renderable =
      | { type: 'object'; obj: MapObject }
      | { type: 'wall'; wall: Wall };

    const renderables: Array<{ depth: number; item: Renderable }> = [];

    // Add objects
    for (const obj of chunk.mapObjects) {
      renderables.push({
        depth: obj.x + obj.y,
        item: { type: 'object', obj }
      });
    }

    // Add walls
    for (const wall of chunk.walls) {
      renderables.push({
        depth: wall.x + wall.y,
        item: { type: 'wall', wall }
      });
    }

    // Sort by depth (back to front)
    renderables.sort((a, b) => a.depth - b.depth);

    // Render in sorted order
    for (const { item } of renderables) {
      if (item.type === 'object') {
        this.renderMapObject(item.obj, chunk, viewport);
      } else {
        this.renderWall(item.wall, chunk, viewport);
      }
    }
  }

  private renderMapObject(obj: MapObject, chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    const worldCoord = chunkLocalToWorld(chunk.coord, { lx: obj.x, ly: obj.y });
    const screen = worldToScreen(worldCoord, viewport);

    // Get the object definition from objectLoader using the gid
    const objDef = objectLoader.getObjectByGid(obj.gid);

    if (objDef?.image) {
      // Calculate draw position - objects are anchored at their base tile
      // The sprite extends upward from the base position
      const scaledWidth = obj.width * viewport.zoom;
      const scaledHeight = obj.height * viewport.zoom;

      // Center horizontally on the tile, align bottom to tile position
      const drawX = screen.sx - scaledWidth / 2;
      const drawY = screen.sy + TILE_HEIGHT * viewport.zoom - scaledHeight;

      this.ctx.drawImage(
        objDef.image,
        0,
        0,
        objDef.image.width,
        objDef.image.height,
        drawX,
        drawY,
        scaledWidth,
        scaledHeight
      );
    }
  }

  private renderWall(wall: Wall, chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    const worldCoord = chunkLocalToWorld(chunk.coord, { lx: wall.x, ly: wall.y });
    const screen = worldToScreen(worldCoord, viewport);

    // Get the object definition from objectLoader using the gid
    const objDef = objectLoader.getObjectByGid(wall.gid);

    if (objDef?.image) {
      const scaledWidth = objDef.image.width * viewport.zoom;
      const scaledHeight = objDef.image.height * viewport.zoom;

      // Bottom vertex of tile
      const bottomVertexX = screen.sx;
      const bottomVertexY = screen.sy + TILE_HEIGHT * viewport.zoom;

      let drawX: number;
      let drawY: number;

      if (wall.edge === 'down') {
        // Bottom-right corner of sprite at bottom vertex
        drawX = bottomVertexX - scaledWidth;
        drawY = bottomVertexY - scaledHeight;
      } else {
        // Bottom-left corner of sprite at bottom vertex (right edge)
        drawX = bottomVertexX;
        drawY = bottomVertexY - scaledHeight;
      }

      this.ctx.drawImage(
        objDef.image,
        0,
        0,
        objDef.image.width,
        objDef.image.height,
        drawX,
        drawY,
        scaledWidth,
        scaledHeight
      );
    }
  }

  private renderEntities(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    for (const entity of chunk.entities) {
      const worldCoord = chunkLocalToWorld(chunk.coord, { lx: entity.x, ly: entity.y });
      const screen = worldToScreen(worldCoord, viewport);

      // Draw entity marker
      const size = 8 * viewport.zoom;
      this.ctx.fillStyle = '#ffd93d';
      this.ctx.strokeStyle = '#000000';
      this.ctx.lineWidth = 1;

      this.ctx.beginPath();
      this.ctx.arc(screen.sx, screen.sy + (TILE_HEIGHT / 2) * viewport.zoom, size, 0, Math.PI * 2);
      this.ctx.fill();
      this.ctx.stroke();

      // Draw entity name
      if (viewport.zoom >= 0.5) {
        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = `${10 * Math.max(1, viewport.zoom)}px sans-serif`;
        this.ctx.textAlign = 'center';
        this.ctx.fillText(
          entity.name || entity.entityId,
          screen.sx,
          screen.sy + (TILE_HEIGHT / 2) * viewport.zoom - size - 4
        );
      }
    }
  }

  private renderPortals(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx || !chunk.portals) return;

    for (const portal of chunk.portals) {
      // Render each tile of the portal
      for (let py = 0; py < portal.height; py++) {
        for (let px = 0; px < portal.width; px++) {
          const worldCoord = chunkLocalToWorld(chunk.coord, { lx: portal.x + px, ly: portal.y + py });
          const screen = worldToScreen(worldCoord, viewport);

          const hw = (TILE_WIDTH / 2) * viewport.zoom;
          const hh = (TILE_HEIGHT / 2) * viewport.zoom;

          // Draw semi-transparent purple diamond
          this.ctx.fillStyle = 'rgba(128, 0, 255, 0.5)';
          this.ctx.beginPath();
          this.ctx.moveTo(screen.sx, screen.sy);
          this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
          this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
          this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
          this.ctx.closePath();
          this.ctx.fill();

          // Draw border
          this.ctx.strokeStyle = 'rgba(180, 0, 255, 0.8)';
          this.ctx.lineWidth = 2;
          this.ctx.stroke();
        }
      }

      // Draw portal label on the first tile
      if (viewport.zoom >= 0.5) {
        const worldCoord = chunkLocalToWorld(chunk.coord, { lx: portal.x, ly: portal.y });
        const screen = worldToScreen(worldCoord, viewport);

        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = `bold ${10 * Math.max(1, viewport.zoom)}px sans-serif`;
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';

        const label = portal.targetMap || 'Portal';
        this.ctx.fillText(
          label,
          screen.sx,
          screen.sy + (TILE_HEIGHT / 2) * viewport.zoom
        );
      }
    }
  }

  private renderGrid(viewport: Viewport): void {
    if (!this.ctx || !this.canvas) return;

    this.ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    this.ctx.lineWidth = 1;

    // Calculate visible world bounds
    const topLeft = { wx: -50, wy: -50 };
    const bottomRight = { wx: 150, wy: 150 };

    for (let y = topLeft.wy; y <= bottomRight.wy; y++) {
      for (let x = topLeft.wx; x <= bottomRight.wx; x++) {
        const screen = worldToScreen({ wx: x, wy: y }, viewport);
        const hw = (TILE_WIDTH / 2) * viewport.zoom;
        const hh = (TILE_HEIGHT / 2) * viewport.zoom;

        this.ctx.beginPath();
        this.ctx.moveTo(screen.sx, screen.sy);
        this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
        this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
        this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
        this.ctx.closePath();
        this.ctx.stroke();
      }
    }
  }

  // Highlight a specific tile (for hover/selection)
  highlightTile(worldCoord: WorldCoord, viewport: Viewport, color: string = '#ffffff', fill: boolean = false): void {
    if (!this.ctx) return;

    const screen = worldToScreen(worldCoord, viewport);
    const hw = (TILE_WIDTH / 2) * viewport.zoom;
    const hh = (TILE_HEIGHT / 2) * viewport.zoom;

    this.ctx.beginPath();
    this.ctx.moveTo(screen.sx, screen.sy);
    this.ctx.lineTo(screen.sx + hw, screen.sy + hh);
    this.ctx.lineTo(screen.sx, screen.sy + TILE_HEIGHT * viewport.zoom);
    this.ctx.lineTo(screen.sx - hw, screen.sy + hh);
    this.ctx.closePath();

    if (fill) {
      this.ctx.fillStyle = color;
      this.ctx.fill();
    } else {
      this.ctx.strokeStyle = color;
      this.ctx.lineWidth = 2;
      this.ctx.stroke();
    }
  }

  // Draw selection rectangle
  renderSelection(start: WorldCoord, end: WorldCoord, viewport: Viewport): void {
    if (!this.ctx) return;

    const minX = Math.min(start.wx, end.wx);
    const maxX = Math.max(start.wx, end.wx);
    const minY = Math.min(start.wy, end.wy);
    const maxY = Math.max(start.wy, end.wy);

    this.ctx.strokeStyle = '#00ff00';
    this.ctx.lineWidth = 2;
    this.ctx.setLineDash([5, 5]);

    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        this.highlightTile({ wx: x, wy: y }, viewport, 'rgba(0, 255, 0, 0.3)');
      }
    }

    this.ctx.setLineDash([]);
  }
}

// Singleton instance
export const isometricRenderer = new IsometricRenderer();
