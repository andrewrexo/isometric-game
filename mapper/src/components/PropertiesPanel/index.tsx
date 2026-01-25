import { useEffect } from 'react';
import { useEditorStore } from '@/state/store';
import { objectLoader } from '@/core/ObjectLoader';
import { interiorStorage } from '@/core/InteriorStorage';
import type { EntitySpawn, Portal, ExitPortal } from '@/types';
import styles from './PropertiesPanel.module.css';

export function PropertiesPanel() {
  const {
    chunks,
    selectedEntitySpawn,
    selectedMapObject,
    selectedPortal,
    selectedExitPortal,
    currentInterior,
    availableInteriors,
    setSelectedEntitySpawn,
    setSelectedMapObject,
    setSelectedPortal,
    setSelectedExitPortal,
    updateEntity,
    removeEntity,
    removeMapObject,
    updatePortal,
    removePortal,
    updateExitPortal,
    removeExitPortal,
    jumpToPortalTarget,
    jumpToExitPortalTarget,
  } = useEditorStore();

  // Get the actual entity spawn from the selection
  const getSelectedEntity = () => {
    if (!selectedEntitySpawn) return null;
    const chunk = chunks.get(
      `${selectedEntitySpawn.chunkCoord.cx},${selectedEntitySpawn.chunkCoord.cy}`
    );
    if (!chunk) return null;
    const spawn = chunk.entities.find((e) => e.id === selectedEntitySpawn.spawnId);
    if (!spawn) return null;
    return { spawn, chunkCoord: selectedEntitySpawn.chunkCoord };
  };

  // Get the actual map object from the selection
  const getSelectedObject = () => {
    if (!selectedMapObject) return null;
    const chunk = chunks.get(
      `${selectedMapObject.chunkCoord.cx},${selectedMapObject.chunkCoord.cy}`
    );
    if (!chunk) return null;
    const obj = chunk.mapObjects.find((o) => o.id === selectedMapObject.objectId);
    if (!obj) return null;
    return { object: obj, chunkCoord: selectedMapObject.chunkCoord };
  };

  // Get the actual portal from the selection
  const getSelectedPortalData = () => {
    if (!selectedPortal) return null;
    const chunk = chunks.get(
      `${selectedPortal.chunkCoord.cx},${selectedPortal.chunkCoord.cy}`
    );
    if (!chunk || !chunk.portals) return null;
    const portal = chunk.portals.find((p) => p.id === selectedPortal.portalId);
    if (!portal) return null;
    return { portal, chunkCoord: selectedPortal.chunkCoord };
  };

  // Get the actual exit portal from the selection (interior mode)
  const getSelectedExitPortalData = () => {
    if (!selectedExitPortal || !currentInterior) return null;
    const portal = currentInterior.exitPortals?.find((p) => p.id === selectedExitPortal.portalId);
    if (!portal) return null;
    return portal;
  };

  const selectedEntity = getSelectedEntity();
  const selectedObject = getSelectedObject();
  const selectedPortalData = getSelectedPortalData();
  const selectedExitPortalData = getSelectedExitPortalData();

  // Delete key handler
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedEntity) {
          e.preventDefault();
          removeEntity(selectedEntity.chunkCoord, selectedEntity.spawn.id);
          setSelectedEntitySpawn(null);
        } else if (selectedObject) {
          e.preventDefault();
          removeMapObject(selectedObject.chunkCoord, selectedObject.object.id);
          setSelectedMapObject(null);
        } else if (selectedPortalData) {
          e.preventDefault();
          removePortal(selectedPortalData.chunkCoord, selectedPortalData.portal.id);
          setSelectedPortal(null);
        } else if (selectedExitPortalData) {
          e.preventDefault();
          removeExitPortal(selectedExitPortalData.id);
          setSelectedExitPortal(null);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedEntity, selectedObject, selectedPortalData, selectedExitPortalData, removeEntity, removeMapObject, removePortal, removeExitPortal, setSelectedEntitySpawn, setSelectedMapObject, setSelectedPortal, setSelectedExitPortal]);

  // Show portal properties
  if (selectedPortalData) {
    const { portal, chunkCoord } = selectedPortalData;

    const handlePortalChange = (field: keyof Portal, value: string | number) => {
      updatePortal(chunkCoord, portal.id, { [field]: value });
    };

    const handleDeletePortal = () => {
      removePortal(chunkCoord, portal.id);
      setSelectedPortal(null);
    };

    const handleJumpToTarget = () => {
      if (portal.targetMap) {
        jumpToPortalTarget(portal);
      }
    };

    // Get spawn points for the selected target map
    const targetSpawnPoints = portal.targetMap
      ? interiorStorage.getSpawnPoints(portal.targetMap)
      : [];

    // Calculate world position
    const worldX = chunkCoord.cx * 32 + portal.x;
    const worldY = chunkCoord.cy * 32 + portal.y;

    return (
      <div className={styles.panel}>
        <div className={styles.title}>Portal Properties</div>
        <div className={styles.content}>
          <div className={styles.field}>
            <label className={styles.label}>Target Map</label>
            {availableInteriors.length > 0 ? (
              <select
                className={styles.select}
                value={portal.targetMap}
                onChange={(e) => handlePortalChange('targetMap', e.target.value)}
              >
                <option value="">Select interior...</option>
                {availableInteriors.map((id) => (
                  <option key={id} value={id}>{id}</option>
                ))}
              </select>
            ) : (
              <input
                type="text"
                className={styles.input}
                value={portal.targetMap}
                placeholder="e.g., test_house"
                onChange={(e) => handlePortalChange('targetMap', e.target.value)}
              />
            )}
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Target Spawn</label>
            {targetSpawnPoints.length > 0 ? (
              <select
                className={styles.select}
                value={portal.targetSpawn}
                onChange={(e) => handlePortalChange('targetSpawn', e.target.value)}
              >
                <option value="">Select spawn point...</option>
                {targetSpawnPoints.map((sp) => (
                  <option key={sp.name} value={sp.name}>{sp.name}</option>
                ))}
              </select>
            ) : (
              <input
                type="text"
                className={styles.input}
                value={portal.targetSpawn}
                placeholder="e.g., entrance"
                onChange={(e) => handlePortalChange('targetSpawn', e.target.value)}
              />
            )}
          </div>

          {portal.targetMap && (
            <button
              className={styles.jumpButton}
              onClick={handleJumpToTarget}
            >
              Jump To Target
            </button>
          )}

          <div className={styles.field}>
            <label className={styles.label}>Width (tiles)</label>
            <input
              type="number"
              className={styles.input}
              value={portal.width}
              min={1}
              onChange={(e) => handlePortalChange('width', Math.max(1, parseInt(e.target.value) || 1))}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Height (tiles)</label>
            <input
              type="number"
              className={styles.input}
              value={portal.height}
              min={1}
              onChange={(e) => handlePortalChange('height', Math.max(1, parseInt(e.target.value) || 1))}
            />
          </div>

          <div className={styles.info}>
            Position: {worldX}, {worldY}
            <br />
            Chunk: {chunkCoord.cx}, {chunkCoord.cy}
            <br />
            Local: {portal.x}, {portal.y}
          </div>

          <div className={styles.actions}>
            <button className={styles.deleteButton} onClick={handleDeletePortal}>
              Delete Portal
            </button>
          </div>
          <div className={styles.hint}>Press Delete to remove</div>
        </div>
      </div>
    );
  }

  // Show exit portal properties (interior mode)
  if (selectedExitPortalData && currentInterior) {
    const portal = selectedExitPortalData;

    const handleExitPortalChange = (field: keyof ExitPortal, value: number) => {
      updateExitPortal(portal.id, { [field]: value });
    };

    const handleDeleteExitPortal = () => {
      removeExitPortal(portal.id);
      setSelectedExitPortal(null);
    };

    const handleJumpToTarget = () => {
      jumpToExitPortalTarget(portal);
    };

    const handleApplyToAll = () => {
      // Apply current targetX/targetY to all exit portals in this interior
      const exitPortals = currentInterior.exitPortals || [];
      for (const p of exitPortals) {
        updateExitPortal(p.id, { targetX: portal.targetX, targetY: portal.targetY });
      }
    };

    return (
      <div className={styles.panel}>
        <div className={styles.title}>Exit Portal Properties</div>
        <div className={styles.content}>
          <div className={styles.field}>
            <label className={styles.label}>Exit Target X (world)</label>
            <input
              type="number"
              className={styles.input}
              value={portal.targetX}
              onChange={(e) => handleExitPortalChange('targetX', parseFloat(e.target.value) || 0)}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Exit Target Y (world)</label>
            <input
              type="number"
              className={styles.input}
              value={portal.targetY}
              onChange={(e) => handleExitPortalChange('targetY', parseFloat(e.target.value) || 0)}
            />
          </div>

          {(portal.targetX !== 0 || portal.targetY !== 0) && (
            <button
              className={styles.jumpButton}
              onClick={handleJumpToTarget}
            >
              Jump To Exit Location
            </button>
          )}

          <button
            className={styles.applyAllButton}
            onClick={handleApplyToAll}
          >
            Apply to All Exits ({currentInterior.exitPortals?.length || 0})
          </button>

          <div className={styles.field}>
            <label className={styles.label}>Width (tiles)</label>
            <input
              type="number"
              className={styles.input}
              value={portal.width}
              min={1}
              onChange={(e) => handleExitPortalChange('width', Math.max(1, parseInt(e.target.value) || 1))}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Height (tiles)</label>
            <input
              type="number"
              className={styles.input}
              value={portal.height}
              min={1}
              onChange={(e) => handleExitPortalChange('height', Math.max(1, parseInt(e.target.value) || 1))}
            />
          </div>

          <div className={styles.info}>
            Position in interior: {portal.x}, {portal.y}
          </div>

          <div className={styles.actions}>
            <button className={styles.deleteButton} onClick={handleDeleteExitPortal}>
              Delete Exit Portal
            </button>
          </div>
          <div className={styles.hint}>Press Delete to remove</div>
        </div>
      </div>
    );
  }

  // Show entity properties
  if (selectedEntity) {
    const { spawn, chunkCoord } = selectedEntity;

    const handleChange = (field: keyof EntitySpawn, value: string | number | boolean) => {
      updateEntity(chunkCoord, spawn.id, { [field]: value });
    };

    const handleDelete = () => {
      removeEntity(chunkCoord, spawn.id);
      setSelectedEntitySpawn(null);
    };

    return (
      <div className={styles.panel}>
        <div className={styles.title}>Entity Properties</div>
        <div className={styles.content}>
          <div className={styles.field}>
            <label className={styles.label}>Entity ID</label>
            <input
              type="text"
              className={styles.input}
              value={spawn.entityId}
              onChange={(e) => handleChange('entityId', e.target.value)}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Name</label>
            <input
              type="text"
              className={styles.input}
              value={spawn.name}
              onChange={(e) => handleChange('name', e.target.value)}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Level</label>
            <input
              type="number"
              className={styles.input}
              value={spawn.level}
              min={1}
              onChange={(e) => handleChange('level', parseInt(e.target.value) || 1)}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Unique ID</label>
            <input
              type="text"
              className={styles.input}
              value={spawn.uniqueId || ''}
              placeholder="Optional"
              onChange={(e) => handleChange('uniqueId', e.target.value)}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Facing</label>
            <select
              className={styles.select}
              value={spawn.facing || ''}
              onChange={(e) => handleChange('facing', e.target.value)}
            >
              <option value="">Default</option>
              <option value="north">North</option>
              <option value="south">South</option>
              <option value="east">East</option>
              <option value="west">West</option>
            </select>
          </div>

          <div className={styles.field}>
            <label className={styles.checkboxLabel}>
              <input
                type="checkbox"
                checked={spawn.respawn ?? false}
                onChange={(e) => handleChange('respawn', e.target.checked)}
              />
              Respawn
            </label>
          </div>

          <div className={styles.info}>
            Position: {spawn.x}, {spawn.y}
            <br />
            Chunk: {chunkCoord.cx}, {chunkCoord.cy}
          </div>

          <div className={styles.actions}>
            <button className={styles.deleteButton} onClick={handleDelete}>
              Delete Entity
            </button>
          </div>
          <div className={styles.hint}>Press Delete to remove</div>
        </div>
      </div>
    );
  }

  // Show object properties
  if (selectedObject) {
    const { object, chunkCoord } = selectedObject;
    const objDef = objectLoader.getObjectByGid(object.gid);

    const handleDelete = () => {
      removeMapObject(chunkCoord, object.id);
      setSelectedMapObject(null);
    };

    return (
      <div className={styles.panel}>
        <div className={styles.title}>Object Properties</div>
        <div className={styles.content}>
          <div className={styles.field}>
            <label className={styles.label}>Type</label>
            <div className={styles.value}>{objDef?.name || `GID: ${object.gid}`}</div>
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Size</label>
            <div className={styles.value}>{object.width} x {object.height} px</div>
          </div>

          <div className={styles.info}>
            Position: {object.x}, {object.y}
            <br />
            Chunk: {chunkCoord.cx}, {chunkCoord.cy}
          </div>

          <div className={styles.actions}>
            <button className={styles.deleteButton} onClick={handleDelete}>
              Delete Object
            </button>
          </div>
          <div className={styles.hint}>Press Delete to remove</div>
        </div>
      </div>
    );
  }

  // Nothing selected - only show title
  return (
    <div className={styles.panel}>
      <div className={styles.title}>Properties</div>
    </div>
  );
}
