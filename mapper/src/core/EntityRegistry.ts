import { parse as parseTOML } from 'smol-toml';
import type { EntityDefinition, EntityRegistry } from '@/types';

interface TOMLEntityStats {
  max_hp?: number;
  damage?: number;
  attack_range?: number;
  aggro_range?: number;
  chase_range?: number;
}

interface TOMLEntityBehaviors {
  hostile?: boolean;
  quest_giver?: boolean;
  merchant?: boolean;
  craftsman?: boolean;
  wander_enabled?: boolean;
}

interface TOMLEntityData {
  display_name?: string;
  sprite?: string;
  animation_type?: string;
  description?: string;
  stats?: TOMLEntityStats;
  behaviors?: TOMLEntityBehaviors;
}

type TOMLDocument = Record<string, TOMLEntityData>;

export class EntityRegistryLoader {
  private registry: EntityRegistry = {
    entities: new Map(),
    byType: {
      hostile: [],
      questGiver: [],
      merchant: [],
      other: [],
    },
  };

  async loadFromFiles(filePaths: string[]): Promise<EntityRegistry> {
    for (const path of filePaths) {
      try {
        await this.loadFile(path);
      } catch (error) {
        console.warn(`Failed to load entity file ${path}:`, error);
      }
    }
    this.categorizeEntities();
    return this.registry;
  }

  async loadFromDirectory(basePath: string): Promise<EntityRegistry> {
    // In a browser environment, we need to know the files ahead of time
    // Load from both npcs and monsters directories
    const entityFiles = [
      // NPCs
      `${basePath}/npcs/villagers.toml`,
      `${basePath}/npcs/merchants.toml`,
      `${basePath}/npcs/quest_givers.toml`,
      // Monsters
      `${basePath}/monsters/pig.toml`,
      `${basePath}/monsters/forest_creatures.toml`,
      `${basePath}/monsters/dangerous_creatures.toml`,
      `${basePath}/monsters/corrupted_creatures.toml`,
      `${basePath}/monsters/creatures.toml`,
      `${basePath}/monsters/enemies.toml`,
    ];

    for (const file of entityFiles) {
      try {
        await this.loadFile(file);
      } catch {
        // File doesn't exist, skip
      }
    }

    this.categorizeEntities();
    return this.registry;
  }

  private async loadFile(path: string): Promise<void> {
    const response = await fetch(path);
    if (!response.ok) {
      throw new Error(`Failed to load: ${response.statusText}`);
    }

    const tomlText = await response.text();
    const parsed = parseTOML(tomlText) as TOMLDocument;

    // Process each entity in the TOML file
    for (const [id, data] of Object.entries(parsed)) {
      // Skip nested tables (like [entity.stats])
      if (typeof data !== 'object' || data === null) continue;
      if (!('display_name' in data) && !('sprite' in data)) continue;

      const entity: EntityDefinition = {
        id,
        displayName: data.display_name || id,
        sprite: data.sprite || id,
        description: data.description || '',
        behaviors: {
          hostile: data.behaviors?.hostile ?? false,
          questGiver: data.behaviors?.quest_giver ?? false,
          merchant: data.behaviors?.merchant ?? false,
          craftsman: data.behaviors?.craftsman ?? false,
        },
      };

      this.registry.entities.set(id, entity);
    }
  }

  private categorizeEntities(): void {
    this.registry.byType = {
      hostile: [],
      questGiver: [],
      merchant: [],
      other: [],
    };

    for (const entity of this.registry.entities.values()) {
      if (entity.behaviors.hostile) {
        this.registry.byType.hostile.push(entity);
      } else if (entity.behaviors.questGiver) {
        this.registry.byType.questGiver.push(entity);
      } else if (entity.behaviors.merchant) {
        this.registry.byType.merchant.push(entity);
      } else {
        this.registry.byType.other.push(entity);
      }
    }

    // Sort each category alphabetically
    for (const category of Object.values(this.registry.byType)) {
      category.sort((a, b) => a.displayName.localeCompare(b.displayName));
    }
  }

  getRegistry(): EntityRegistry {
    return this.registry;
  }

  getEntity(id: string): EntityDefinition | undefined {
    return this.registry.entities.get(id);
  }

  getAllEntities(): EntityDefinition[] {
    return Array.from(this.registry.entities.values());
  }

  searchEntities(query: string): EntityDefinition[] {
    const lowerQuery = query.toLowerCase();
    return this.getAllEntities().filter(
      (entity) =>
        entity.id.toLowerCase().includes(lowerQuery) ||
        entity.displayName.toLowerCase().includes(lowerQuery) ||
        entity.description.toLowerCase().includes(lowerQuery)
    );
  }
}

// Singleton instance
export const entityRegistryLoader = new EntityRegistryLoader();
