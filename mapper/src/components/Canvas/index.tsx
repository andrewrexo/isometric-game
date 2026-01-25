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
    selectedPortal,
    showGrid,
    showChunkBounds,
    showCollision,
    showEntities,
    showMapObjects,
    showPortals,
    visibleLayers,
    editorMode,
    currentInterior,
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
    addPortal,
    setSelectedTileId,
    findEntityAtWorld,
    setSelectedEntitySpawn,
    findMapObjectAtWorld,
    setSelectedMapObject,
    findPortalAtWorld,
    setSelectedPortal,
    toggleWall,
    setInteriorTile,
    toggleInteriorCollision,
    fillInteriorTiles,
    addInteriorEntity,
    removeInteriorEntity,
    addInteriorMapObject,
    removeInteriorMapObject,
    toggleInteriorWall,
    addSpawnPoint,
    removeSpawnPoint,
    addExitPortal,
    removeExitPortal,
    findExitPortalAt,
    setSelectedExitPortal,
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
      showPortals,
      visibleLayers,
    });
  }, [showGrid, showChunkBounds, showCollision, showEntities, showMapObjects, showPortals, visibleLayers]);

  // Render loop
  useEffect(() => {
    let animationId: number;

    const render = () => {
      if (editorMode === 'interior' && currentInterior) {
        // Render interior map
        isometricRenderer.renderInterior(currentInterior, viewport);
      } else {
        // Render overworld chunks
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

        // Highlight selected portal
        if (selectedPortal) {
          const chunk = chunks.get(`${selectedPortal.chunkCoord.cx},${selectedPortal.chunkCoord.cy}`);
          if (chunk && chunk.portals) {
            const portal = chunk.portals.find((p) => p.id === selectedPortal.portalId);
            if (portal) {
              const baseX = selectedPortal.chunkCoord.cx * 32;
              const baseY = selectedPortal.chunkCoord.cy * 32;
              for (let py = 0; py < portal.height; py++) {
                for (let px = 0; px < portal.width; px++) {
                  isometricRenderer.highlightTile(
                    { wx: baseX + portal.x + px, wy: baseY + portal.y + py },
                    viewport,
                    'rgba(200, 0, 255, 0.6)',
                    true
                  );
                }
              }
            }
          }
        }
      }

      // Highlight hovered tile (works for both modes)
      if (hoveredTile) {
        isometricRenderer.highlightTile(hoveredTile, viewport, '#ffffff');
      }

      animationId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animationId);
    };
  }, [chunks, viewport, hoveredTile, selectedTiles, selectedPortal, editorMode, currentInterior]);

  // Handle tool action at position
  const handleToolAction = useCallback(
    (clientX: number, clientY: number) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const screenX = clientX - rect.left;
      const screenY = clientY - rect.top;
      const worldTile = screenToWorldTile({ sx: screenX, sy: screenY }, viewport);

      // Interior mode handling
      if (editorMode === 'interior' && currentInterior) {
        // Check bounds
        if (worldTile.wx < 0 || worldTile.wx >= currentInterior.width ||
            worldTile.wy < 0 || worldTile.wy >= currentInterior.height) {
          return; // Outside interior bounds
        }

        switch (activeTool) {
          case Tool.Paint:
            if (activeLayer === Layer.Ground || activeLayer === Layer.Objects || activeLayer === Layer.Overhead) {
              setInteriorTile(worldTile.wx, worldTile.wy, activeLayer, selectedTileId);
            }
            break;
          case Tool.Eraser:
            if (activeLayer === Layer.Entities) {
              const entity = currentInterior.entities.find(
                (e) => e.x === worldTile.wx && e.y === worldTile.wy
              );
              if (entity) {
                removeInteriorEntity(entity.id);
              }
            } else if (activeLayer === Layer.MapObjects) {
              const obj = currentInterior.mapObjects.find(
                (o) => o.x === worldTile.wx && o.y === worldTile.wy
              );
              if (obj) {
                removeInteriorMapObject(obj.id);
              }
            } else if (activeLayer === Layer.Ground || activeLayer === Layer.Objects || activeLayer === Layer.Overhead) {
              setInteriorTile(worldTile.wx, worldTile.wy, activeLayer, 0);
            }
            break;
          case Tool.Collision:
            toggleInteriorCollision(worldTile.wx, worldTile.wy);
            break;
          case Tool.Fill:
            if (activeLayer === Layer.Ground || activeLayer === Layer.Objects || activeLayer === Layer.Overhead) {
              fillInteriorTiles(worldTile.wx, worldTile.wy, activeLayer, selectedTileId);
            }
            break;
          case Tool.Entity:
            if (selectedEntityId) {
              addInteriorEntity(worldTile.wx, worldTile.wy, selectedEntityId);
            }
            break;
          case Tool.Object:
            if (selectedObjectId) {
              const objDef = objectLoader.getObject(selectedObjectId);
              if (objDef) {
                addInteriorMapObject(worldTile.wx, worldTile.wy, selectedObjectId, objDef.width, objDef.height);
              }
            }
            break;
          case Tool.Eyedropper: {
            const index = worldTile.wy * currentInterior.width + worldTile.wx;
            const layerKey =
              activeLayer === Layer.Ground
                ? 'ground'
                : activeLayer === Layer.Objects
                  ? 'objects'
                  : 'overhead';
            if (layerKey === 'ground' || layerKey === 'objects' || layerKey === 'overhead') {
              const tileId = currentInterior.layers[layerKey][index];
              if (tileId > 0) {
                setSelectedTileId(tileId);
              }
            }
            break;
          }
          case Tool.WallDown:
          case Tool.WallRight: {
            if (selectedObjectId) {
              const wallDef = objectLoader.getWall(selectedObjectId);
              const objDef = wallDef || objectLoader.getObject(selectedObjectId);
              if (objDef) {
                const edge = activeTool === Tool.WallDown ? 'down' : 'right';
                const gid = wallDef
                  ? objectLoader.wallIdToGid(selectedObjectId)
                  : objectLoader.idToGid(selectedObjectId);
                toggleInteriorWall(worldTile.wx, worldTile.wy, edge, gid);
              }
            }
            break;
          }
          case Tool.SpawnPoint: {
            // Check if spawn point exists at this location
            const existingSpawn = currentInterior.spawnPoints.find(
              (sp) => sp.x === worldTile.wx && sp.y === worldTile.wy
            );
            if (existingSpawn) {
              removeSpawnPoint(existingSpawn.name);
            } else {
              // Prompt for spawn point name
              const name = prompt('Enter spawn point name:', `spawn_${currentInterior.spawnPoints.length}`);
              if (name) {
                addSpawnPoint(name, worldTile.wx, worldTile.wy);
              }
            }
            break;
          }
          case Tool.ExitPortal: {
            // Check if exit portal exists at this location
            const existingExit = findExitPortalAt(worldTile.wx, worldTile.wy);
            if (existingExit) {
              // Select the existing exit portal
              setSelectedExitPortal({ portalId: existingExit.id });
            } else {
              // Add new exit portal and select it
              const newPortal = addExitPortal(worldTile.wx, worldTile.wy);
              setSelectedExitPortal({ portalId: newPortal.id });
            }
            break;
          }
        }
        return;
      }

      // Overworld mode handling
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
        case Tool.WallDown:
        case Tool.WallRight: {
          // Place or remove wall at position
          if (selectedObjectId) {
            // Check walls first, then objects
            const wallDef = objectLoader.getWall(selectedObjectId);
            const objDef = wallDef || objectLoader.getObject(selectedObjectId);
            if (objDef) {
              const edge = activeTool === Tool.WallDown ? 'down' : 'right';
              // Use wall GID if it's a wall, otherwise object GID
              const gid = wallDef
                ? objectLoader.wallIdToGid(selectedObjectId)
                : objectLoader.idToGid(selectedObjectId);
              toggleWall(worldTile, edge, gid);
            }
          }
          break;
        }
        case Tool.Portal: {
          // First check if there's an existing portal to select
          const existingPortal = findPortalAtWorld(worldTile);
          if (existingPortal) {
            setSelectedPortal({
              chunkCoord: existingPortal.chunkCoord,
              portalId: existingPortal.portal.id,
            });
          } else {
            // No existing portal, place a new one
            const newPortal = addPortal(worldTile);
            setSelectedPortal({
              chunkCoord: useEditorStore.getState().getChunk({
                cx: Math.floor(worldTile.wx / 32),
                cy: Math.floor(worldTile.wy / 32),
              })?.coord || { cx: Math.floor(worldTile.wx / 32), cy: Math.floor(worldTile.wy / 32) },
              portalId: newPortal.id,
            });
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
      editorMode,
      currentInterior,
      setTile,
      toggleCollision,
      fillTiles,
      fillSelectedTiles,
      addEntity,
      removeEntity,
      addMapObject,
      removeMapObject,
      addPortal,
      setSelectedTileId,
      findEntityAtWorld,
      setSelectedEntitySpawn,
      findMapObjectAtWorld,
      setSelectedMapObject,
      findPortalAtWorld,
      setSelectedPortal,
      toggleWall,
      setInteriorTile,
      toggleInteriorCollision,
      fillInteriorTiles,
      addInteriorEntity,
      removeInteriorEntity,
      addInteriorMapObject,
      removeInteriorMapObject,
      toggleInteriorWall,
      addSpawnPoint,
      removeSpawnPoint,
      addExitPortal,
      removeExitPortal,
      findExitPortalAt,
      setSelectedExitPortal,
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
      {editorMode === 'interior' && currentInterior && (
        <div className={styles.modeIndicator}>
          INTERIOR MODE
          <span className={styles.interiorName}>{currentInterior.name}</span>
        </div>
      )}
      {hoveredTile && (
        <div className={styles.coords}>
          {hoveredTile.wx}, {hoveredTile.wy}
        </div>
      )}
    </div>
  );
}
