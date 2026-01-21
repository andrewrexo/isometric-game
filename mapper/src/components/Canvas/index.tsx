import { useEffect, useRef, useCallback, useState } from 'react';
import { useEditorStore } from '@/state/store';
import { isometricRenderer } from '@/core/IsometricRenderer';
import { screenToWorldTile } from '@/core/coords';
import { Tool, Layer } from '@/types';
import { history } from '@/core/History';
import { objectLoader } from '@/core/ObjectLoader';
import styles from './Canvas.module.css';

export function Canvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isPanning, setIsPanning] = useState(false);
  const [isPainting, setIsPainting] = useState(false);
  const lastMousePos = useRef({ x: 0, y: 0 });

  const {
    chunks,
    viewport,
    hoveredTile,
    activeTool,
    activeLayer,
    selectedTileId,
    selectedEntityId,
    selectedObjectId,
    selectedTiles,
    showGrid,
    showChunkBounds,
    showCollision,
    showEntities,
    showMapObjects,
    visibleLayers,
    pan,
    zoom,
    setHoveredTile,
    setTile,
    toggleCollision,
    fillTiles,
    magicWandSelect,
    fillSelectedTiles,
    addEntity,
    removeEntity,
    addMapObject,
    removeMapObject,
    setSelectedTileId,
    findEntityAtWorld,
    setSelectedEntitySpawn,
    findMapObjectAtWorld,
    setSelectedMapObject,
  } = useEditorStore();

  // Setup canvas and renderer
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    isometricRenderer.attach(canvas);

    const resizeObserver = new ResizeObserver(() => {
      if (containerRef.current) {
        canvas.width = containerRef.current.clientWidth;
        canvas.height = containerRef.current.clientHeight;
      }
    });

    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
      canvas.width = containerRef.current.clientWidth;
      canvas.height = containerRef.current.clientHeight;
    }

    return () => {
      resizeObserver.disconnect();
      isometricRenderer.detach();
    };
  }, []);

  // Arrow key camera movement
  const heldKeys = useRef<Set<string>>(new Set());

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
        heldKeys.current.add(e.key);
        e.preventDefault();
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      heldKeys.current.delete(e.key);
    };

    let animationId: number;
    const panLoop = () => {
      const panAmount = 5;
      let dx = 0;
      let dy = 0;

      if (heldKeys.current.has('ArrowUp')) dy += panAmount;
      if (heldKeys.current.has('ArrowDown')) dy -= panAmount;
      if (heldKeys.current.has('ArrowLeft')) dx += panAmount;
      if (heldKeys.current.has('ArrowRight')) dx -= panAmount;

      if (dx !== 0 || dy !== 0) {
        pan(dx, dy);
      }

      animationId = requestAnimationFrame(panLoop);
    };

    animationId = requestAnimationFrame(panLoop);
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      cancelAnimationFrame(animationId);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [pan]);

  // Update renderer options
  useEffect(() => {
    isometricRenderer.setOptions({
      showGrid,
      showChunkBounds,
      showCollision,
      showEntities,
      showMapObjects,
      visibleLayers,
    });
  }, [showGrid, showChunkBounds, showCollision, showEntities, showMapObjects, visibleLayers]);

  // Render loop
  useEffect(() => {
    let animationId: number;

    const render = () => {
      const allChunks = Array.from(chunks.values());
      isometricRenderer.render(allChunks, viewport);

      // Highlight selected tiles (magic wand selection)
      if (selectedTiles.size > 0) {
        for (const tileKey of selectedTiles) {
          const [wxStr, wyStr] = tileKey.split(',');
          const wx = parseInt(wxStr, 10);
          const wy = parseInt(wyStr, 10);
          isometricRenderer.highlightTile({ wx, wy }, viewport, 'rgba(0, 150, 255, 0.4)', true);
        }
      }

      // Highlight hovered tile
      if (hoveredTile) {
        isometricRenderer.highlightTile(hoveredTile, viewport, '#ffffff');
      }

      animationId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animationId);
    };
  }, [chunks, viewport, hoveredTile, selectedTiles]);

  // Handle tool action at position
  const handleToolAction = useCallback(
    (clientX: number, clientY: number) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const screenX = clientX - rect.left;
      const screenY = clientY - rect.top;
      const worldTile = screenToWorldTile({ sx: screenX, sy: screenY }, viewport);

      switch (activeTool) {
        case Tool.Paint:
          if (activeLayer === Layer.Ground || activeLayer === Layer.Objects || activeLayer === Layer.Overhead) {
            // If there's an active selection, fill all selected tiles
            if (selectedTiles.size > 0) {
              fillSelectedTiles(activeLayer, selectedTileId);
            } else {
              setTile(worldTile, activeLayer, selectedTileId);
            }
          }
          break;
        case Tool.Eraser: {
          if (activeLayer === Layer.Entities) {
            // Erase entity at position
            const entityAtPos = findEntityAtWorld(worldTile);
            if (entityAtPos) {
              removeEntity(entityAtPos.chunkCoord, entityAtPos.entity.id);
            }
          } else if (activeLayer === Layer.MapObjects) {
            // Erase map object at position
            const objectAtPos = findMapObjectAtWorld(worldTile);
            if (objectAtPos) {
              removeMapObject(objectAtPos.chunkCoord, objectAtPos.object.id);
            }
          } else if (activeLayer === Layer.Ground || activeLayer === Layer.Objects || activeLayer === Layer.Overhead) {
            // Erase tile
            setTile(worldTile, activeLayer, 0);
          }
          break;
        }
        case Tool.Collision:
          toggleCollision(worldTile);
          break;
        case Tool.Fill:
          if (activeLayer === Layer.Ground || activeLayer === Layer.Objects || activeLayer === Layer.Overhead) {
            fillTiles(worldTile, activeLayer, selectedTileId);
          }
          break;
        case Tool.MagicWand:
          if (activeLayer !== Layer.Collision && activeLayer !== Layer.Entities && activeLayer !== Layer.MapObjects) {
            magicWandSelect(worldTile, activeLayer);
          }
          break;
        case Tool.Entity: {
          // First check if there's an existing entity to select
          const existingEntity = findEntityAtWorld(worldTile);
          if (existingEntity) {
            setSelectedEntitySpawn({
              chunkCoord: existingEntity.chunkCoord,
              spawnId: existingEntity.entity.id,
            });
          } else if (selectedEntityId) {
            // No existing entity, place a new one
            addEntity(worldTile, selectedEntityId);
          }
          break;
        }
        case Tool.Object: {
          // First check if there's an existing object to select
          const existingObject = findMapObjectAtWorld(worldTile);
          if (existingObject) {
            setSelectedMapObject({
              chunkCoord: existingObject.chunkCoord,
              objectId: existingObject.object.id,
            });
          } else if (selectedObjectId) {
            // No existing object, place a new one
            const objDef = objectLoader.getObject(selectedObjectId);
            if (objDef) {
              addMapObject(worldTile, selectedObjectId, objDef.width, objDef.height);
            }
          }
          break;
        }
        case Tool.Select: {
          // Select tool can select entities or objects
          const entityAtPos = findEntityAtWorld(worldTile);
          if (entityAtPos) {
            setSelectedEntitySpawn({
              chunkCoord: entityAtPos.chunkCoord,
              spawnId: entityAtPos.entity.id,
            });
            setSelectedMapObject(null);
          } else {
            const objectAtPos = findMapObjectAtWorld(worldTile);
            if (objectAtPos) {
              setSelectedMapObject({
                chunkCoord: objectAtPos.chunkCoord,
                objectId: objectAtPos.object.id,
              });
              setSelectedEntitySpawn(null);
            } else {
              // Nothing at position, clear selection
              setSelectedEntitySpawn(null);
              setSelectedMapObject(null);
            }
          }
          break;
        }
        case Tool.Eyedropper: {
          // Pick tile from clicked position
          const chunk = useEditorStore.getState().getChunk({
            cx: Math.floor(worldTile.wx / 32),
            cy: Math.floor(worldTile.wy / 32),
          });
          if (chunk) {
            const lx = ((worldTile.wx % 32) + 32) % 32;
            const ly = ((worldTile.wy % 32) + 32) % 32;
            const index = ly * 32 + lx;
            const layerKey =
              activeLayer === Layer.Ground
                ? 'ground'
                : activeLayer === Layer.Objects
                  ? 'objects'
                  : 'overhead';
            if (layerKey === 'ground' || layerKey === 'objects' || layerKey === 'overhead') {
              const tileId = chunk.layers[layerKey][index];
              if (tileId > 0) {
                setSelectedTileId(tileId);
              }
            }
          }
          break;
        }
      }
    },
    [
      viewport,
      activeTool,
      activeLayer,
      selectedTileId,
      selectedEntityId,
      selectedObjectId,
      selectedTiles,
      setTile,
      toggleCollision,
      fillTiles,
      fillSelectedTiles,
      addEntity,
      removeEntity,
      addMapObject,
      removeMapObject,
      setSelectedTileId,
      findEntityAtWorld,
      setSelectedEntitySpawn,
      findMapObjectAtWorld,
      setSelectedMapObject,
    ]
  );

  // Mouse event handlers
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      lastMousePos.current = { x: e.clientX, y: e.clientY };

      // Middle mouse or space+left click for panning
      if (e.button === 1 || (e.button === 0 && e.shiftKey)) {
        setIsPanning(true);
        e.preventDefault();
        return;
      }

      // Left click for tool action
      if (e.button === 0) {
        setIsPainting(true);
        if (activeTool === Tool.Paint || activeTool === Tool.Eraser) {
          history.beginGroup(`${activeTool} stroke`);
        }
        handleToolAction(e.clientX, e.clientY);
      }
    },
    [activeTool, handleToolAction]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const screenX = e.clientX - rect.left;
      const screenY = e.clientY - rect.top;
      const worldTile = screenToWorldTile({ sx: screenX, sy: screenY }, viewport);
      setHoveredTile(worldTile);

      if (isPanning) {
        const dx = e.clientX - lastMousePos.current.x;
        const dy = e.clientY - lastMousePos.current.y;
        pan(dx, dy);
        lastMousePos.current = { x: e.clientX, y: e.clientY };
        return;
      }

      if (isPainting && (activeTool === Tool.Paint || activeTool === Tool.Eraser)) {
        handleToolAction(e.clientX, e.clientY);
      }
    },
    [viewport, isPanning, isPainting, activeTool, pan, setHoveredTile, handleToolAction]
  );

  const handleMouseUp = useCallback(() => {
    if (isPainting && (activeTool === Tool.Paint || activeTool === Tool.Eraser)) {
      history.endGroup();
    }
    setIsPanning(false);
    setIsPainting(false);
  }, [isPainting, activeTool]);

  const handleMouseLeave = useCallback(() => {
    setHoveredTile(null);
    if (isPainting && (activeTool === Tool.Paint || activeTool === Tool.Eraser)) {
      history.endGroup();
    }
    setIsPanning(false);
    setIsPainting(false);
  }, [isPainting, activeTool, setHoveredTile]);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      const factor = e.deltaY > 0 ? 0.9 : 1.1;
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        zoom(factor, e.clientX - rect.left, e.clientY - rect.top);
      }
    },
    [zoom]
  );


  return (
    <div ref={containerRef} className={styles.container}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        onWheel={handleWheel}
        onContextMenu={(e) => e.preventDefault()}
      />
      {hoveredTile && (
        <div className={styles.coords}>
          {hoveredTile.wx}, {hoveredTile.wy}
        </div>
      )}
    </div>
  );
}
