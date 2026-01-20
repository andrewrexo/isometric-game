import type { Chunk, Viewport, WorldCoord } from '@/types';
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

    // Render map objects (trees, rocks, etc.)
    if (this.options.showMapObjects) {
      for (const chunk of sortedChunks) {
        this.renderMapObjects(chunk, viewport);
      }
    }

    if (this.options.showEntities) {
      for (const chunk of sortedChunks) {
        this.renderEntities(chunk, viewport);
      }
    }

    if (this.options.showGrid) {
      this.renderGrid(viewport);
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

  private renderMapObjects(chunk: Chunk, viewport: Viewport): void {
    if (!this.ctx) return;

    // Sort objects by y position for proper depth ordering
    const sortedObjects = [...chunk.mapObjects].sort((a, b) => {
      // Objects with lower y (further back) should render first
      return (a.x + a.y) - (b.x + b.y);
    });

    for (const obj of sortedObjects) {
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
