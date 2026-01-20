import { useEffect } from 'react';
import { useEditorStore } from '@/state/store';
import { tilesetLoader } from '@/core/TilesetLoader';
import { entityRegistryLoader } from '@/core/EntityRegistry';
import { objectLoader } from '@/core/ObjectLoader';
import { chunkManager } from '@/core/ChunkManager';
import { chunkKey } from '@/core/coords';
import { MenuBar } from '@/components/MenuBar';
import { Toolbar } from '@/components/Toolbar';
import { Canvas } from '@/components/Canvas';
import { TilePalette } from '@/components/TilePalette';
import { ObjectPalette } from '@/components/ObjectPalette';
import { LayerPanel } from '@/components/LayerPanel';
import { EntityPanel } from '@/components/EntityPanel';
import { PropertiesPanel } from '@/components/PropertiesPanel';
import './App.css';

function App() {
  const {
    setTilesets,
    setEntityRegistry,
    setChunks,
    setWorldBounds,
    setLoading,
    isLoading,
    loadingMessage,
  } = useEditorStore();

  // Initialize app
  useEffect(() => {
    const init = async () => {
      setLoading(true, 'Loading configuration...');

      try {
        // Load config
        const config = await tilesetLoader.loadConfig('/mapper-config.json');

        // Load tilesets
        setLoading(true, 'Loading tilesets...');
        await tilesetLoader.loadTilesets(config.tilesets);
        setTilesets(tilesetLoader.getAllTilesets());

        // Load objects (trees, rocks, etc.)
        if (config.objects) {
          setLoading(true, 'Loading objects...');
          await objectLoader.loadObjects(config.objects);
        }

        // Load entity registry
        setLoading(true, 'Loading entities...');
        try {
          const registry = await entityRegistryLoader.loadFromDirectory('/entities/npcs');
          setEntityRegistry(registry);
        } catch {
          // If TOML files can't be loaded, create empty registry
          console.warn('Could not load entity TOML files, using empty registry');
          setEntityRegistry({
            entities: new Map(),
            byType: { hostile: [], questGiver: [], merchant: [], other: [] },
          });
        }

        // Discover and load chunks
        setLoading(true, 'Loading map chunks...');

        // Try to load known chunks
        const knownChunks = [
          { cx: 0, cy: 0 },
          { cx: 0, cy: -1 },
          { cx: 1, cy: 0 },
          { cx: -1, cy: 0 },
          { cx: -1, cy: -1 },
          { cx: -2, cy: 0 },
        ];

        for (const coord of knownChunks) {
          try {
            // Try loading from public assets (would need chunks copied there)
            const chunk = await chunkManager.loadChunk(
              `/maps/chunk_${coord.cx}_${coord.cy}.json`,
              coord
            );
            if (chunk) {
              chunkManager.addChunk(chunk);
            }
          } catch {
            // Chunk doesn't exist, that's fine
          }
        }

        // If no chunks loaded, create a default chunk at origin
        if (chunkManager.getAllChunks().length === 0) {
          chunkManager.createEmptyChunk({ cx: 0, cy: 0 });
        }

        // Update store with loaded chunks
        const chunks = new Map<string, ReturnType<typeof chunkManager.getChunk>>();
        for (const chunk of chunkManager.getAllChunks()) {
          chunks.set(chunkKey(chunk.coord), chunk);
        }
        setChunks(chunks as Map<string, NonNullable<ReturnType<typeof chunkManager.getChunk>>>);
        setWorldBounds(chunkManager.getBounds());

        setLoading(false);
      } catch (error) {
        console.error('Failed to initialize:', error);
        setLoading(false);
      }
    };

    init();
  }, [setTilesets, setEntityRegistry, setChunks, setWorldBounds, setLoading]);

  // Keyboard shortcuts for tools
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      const store = useEditorStore.getState();

      switch (e.key.toLowerCase()) {
        case 'v':
          store.setActiveTool('select');
          break;
        case 'b':
          store.setActiveTool('paint');
          break;
        case 'g':
          store.setActiveTool('fill');
          break;
        case 'e':
          store.setActiveTool('eraser');
          break;
        case 'c':
          store.setActiveTool('collision');
          break;
        case 'n':
          store.setActiveTool('entity');
          break;
        case 'o':
          store.setActiveTool('object');
          break;
        case 'i':
          store.setActiveTool('eyedropper');
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  if (isLoading) {
    return (
      <div className="loading">
        <div className="loading-spinner" />
        <div className="loading-message">{loadingMessage}</div>
      </div>
    );
  }

  return (
    <div className="app">
      <MenuBar />
      <div className="main">
        <div className="sidebar left">
          <Toolbar />
          <TilePalette />
          <ObjectPalette />
          <LayerPanel />
        </div>
        <Canvas />
        <div className="sidebar right">
          <EntityPanel />
          <PropertiesPanel />
        </div>
      </div>
    </div>
  );
}

export default App;
