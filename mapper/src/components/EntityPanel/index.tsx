import { useState } from 'react';
import { useEditorStore } from '@/state/store';
import styles from './EntityPanel.module.css';

export function EntityPanel() {
  const { entityRegistry, selectedEntityId, setSelectedEntityId } = useEditorStore();
  const [search, setSearch] = useState('');
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(
    new Set(['hostile', 'questGiver', 'merchant', 'other'])
  );

  if (!entityRegistry) {
    return (
      <div className={styles.panel}>
        <div className={styles.title}>Entities</div>
        <div className={styles.empty}>Loading entities...</div>
      </div>
    );
  }

  const toggleGroup = (group: string) => {
    const newExpanded = new Set(expandedGroups);
    if (newExpanded.has(group)) {
      newExpanded.delete(group);
    } else {
      newExpanded.add(group);
    }
    setExpandedGroups(newExpanded);
  };

  const filterEntities = (entities: typeof entityRegistry.byType.hostile) => {
    if (!search) return entities;
    const lowerSearch = search.toLowerCase();
    return entities.filter(
      (e) =>
        e.id.toLowerCase().includes(lowerSearch) ||
        e.displayName.toLowerCase().includes(lowerSearch)
    );
  };

  const groups = [
    { key: 'hostile', label: 'Hostile', entities: filterEntities(entityRegistry.byType.hostile) },
    { key: 'questGiver', label: 'Quest Givers', entities: filterEntities(entityRegistry.byType.questGiver) },
    { key: 'merchant', label: 'Merchants', entities: filterEntities(entityRegistry.byType.merchant) },
    { key: 'other', label: 'Other', entities: filterEntities(entityRegistry.byType.other) },
  ];

  return (
    <div className={styles.panel}>
      <div className={styles.title}>Entities</div>
      <input
        type="text"
        className={styles.search}
        placeholder="Search entities..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />
      <div className={styles.groups}>
        {groups.map((group) => (
          <div key={group.key} className={styles.group}>
            <button
              className={styles.groupHeader}
              onClick={() => toggleGroup(group.key)}
            >
              <span className={styles.arrow}>
                {expandedGroups.has(group.key) ? '▼' : '▶'}
              </span>
              {group.label} ({group.entities.length})
            </button>
            {expandedGroups.has(group.key) && (
              <div className={styles.entities}>
                {group.entities.map((entity) => (
                  <button
                    key={entity.id}
                    className={`${styles.entity} ${selectedEntityId === entity.id ? styles.selected : ''}`}
                    onClick={() => setSelectedEntityId(entity.id)}
                    title={entity.description}
                  >
                    <span className={styles.entityName}>{entity.displayName}</span>
                    <span className={styles.entityId}>{entity.id}</span>
                  </button>
                ))}
                {group.entities.length === 0 && (
                  <div className={styles.empty}>No entities</div>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
