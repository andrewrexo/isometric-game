import { useEffect } from 'react';
import { useEditorStore } from '@/state/store';
import { objectLoader } from '@/core/ObjectLoader';
import type { EntitySpawn } from '@/types';
import styles from './PropertiesPanel.module.css';

export function PropertiesPanel() {
  const {
    chunks,
    selectedEntitySpawn,
    selectedMapObject,
    setSelectedEntitySpawn,
    setSelectedMapObject,
    updateEntity,
    removeEntity,
    removeMapObject,
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

  const selectedEntity = getSelectedEntity();
  const selectedObject = getSelectedObject();

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
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedEntity, selectedObject, removeEntity, removeMapObject, setSelectedEntitySpawn, setSelectedMapObject]);

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
