import { useEffect, useState, useRef, useCallback } from 'react';
import { useEditorStore } from '@/state/store';
import { tilesetLoader } from '@/core/TilesetLoader';
import { entityRegistryLoader } from '@/core/EntityRegistry';
import { objectLoader } from '@/core/ObjectLoader';
import { chunkManager } from '@/core/ChunkManager';
import { chunkKey } from '@/core/coords';
import { storage } from '@/core/Storage';
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
  const [leftSidebarWidth, setLeftSidebarWidth] = useState(250);
  const [rightSidebarWidth, setRightSidebarWidth] = useState(250);
  const [isResizing, setIsResizing] = useState<'left' | 'right' | null>(null);
  const resizeStartX = useRef(0);
  const resizeStartWidth = useRef(0);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isResizing) return;

    const delta = e.clientX - resizeStartX.current;
    const newWidth = isResizing === 'left'
      ? resizeStartWidth.current + delta
      : resizeStartWidth.current - delta;

    const clampedWidth = Math.max(180, Math.min(500, newWidth));

    if (isResizing === 'left') {
      setLeftSidebarWidth(clampedWidth);
    } else {
      setRightSidebarWidth(clampedWidth);
    }
  }, [isResizing]);

  const handleMouseUp = useCallback(() => {
    setIsResizing(null);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, []);

  useEffect(() => {
    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';

      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isResizing, handleMouseMove, handleMouseUp]);

  const startResize = (side: 'left' | 'right', e: React.MouseEvent) => {
    setIsResizing(side);
    resizeStartX.current = e.clientX;
    resizeStartWidth.current = side === 'left' ? leftSidebarWidth : rightSidebarWidth;
  };

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
          const registry = await entityRegistryLoader.loadFromDirectory('/entities');
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

        // Check if we have saved data in IndexedDB
        const hasStoredData = await storage.hasStoredData();

        if (hasStoredData) {
          setLoading(true, 'Loading saved map data...');
          const storedChunks = await storage.loadAllChunks();

          if (storedChunks.size > 0) {
            // Use stored data
            setChunks(storedChunks, true); // skipAutoSave since this is loading

            // Calculate bounds from stored chunks
            let minCx = Infinity, maxCx = -Infinity;
            let minCy = Infinity, maxCy = -Infinity;
            for (const chunk of storedChunks.values()) {
              minCx = Math.min(minCx, chunk.coord.cx);
              maxCx = Math.max(maxCx, chunk.coord.cx);
              minCy = Math.min(minCy, chunk.coord.cy);
              maxCy = Math.max(maxCy, chunk.coord.cy);
            }
            setWorldBounds({
              minCx: minCx === Infinity ? 0 : minCx,
              maxCx: maxCx === -Infinity ? 0 : maxCx,
              minCy: minCy === Infinity ? 0 : minCy,
              maxCy: maxCy === -Infinity ? 0 : maxCy,
            });

            console.log(`Loaded ${storedChunks.size} chunks from local storage`);
            setLoading(false);
            return;
          }
        }

        // No stored data, load from server
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
      // Ignore if typing in an input
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      const store = useEditorStore.getState();

      // Handle Escape - clear selection
      if (e.key === 'Escape') {
        store.clearSelectedTiles();
        return;
      }

      // Handle Ctrl/Cmd shortcuts
      if (e.ctrlKey || e.metaKey) {
        if (e.key === 'z') {
          e.preventDefault();
          if (e.shiftKey) {
            store.redo();
          } else {
            store.undo();
          }
          return;
        } else if (e.key === 'y') {
          e.preventDefault();
          store.redo();
          return;
        }
      }

      // Tool shortcuts
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
        case 'w':
          store.setActiveTool('magicWand');
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
        <div className="sidebar left" style={{ width: leftSidebarWidth }}>
          <Toolbar />
          <TilePalette />
          <ObjectPalette />
          <LayerPanel />
        </div>
        <div
          className="resize-handle"
          onMouseDown={(e) => startResize('left', e)}
        />
        <Canvas />
        <div
          className="resize-handle"
          onMouseDown={(e) => startResize('right', e)}
        />
        <div className="sidebar right" style={{ width: rightSidebarWidth }}>
          <EntityPanel />
          <PropertiesPanel />
        </div>
      </div>
    </div>
  );
}

export default App;
