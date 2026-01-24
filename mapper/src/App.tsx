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
    setConnected,
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

        // Load walls
        if (config.walls) {
          setLoading(true, 'Loading walls...');
          await objectLoader.loadWalls(config.walls);
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

        // Load chunks from server (falls back to local IndexedDB)
        setLoading(true, 'Loading map data...');
        const loadedChunks = await storage.loadAllChunks();
        setConnected(storage.isConnected);

        if (loadedChunks.size > 0) {
          setChunks(loadedChunks, true); // skipAutoSave since this is loading

          // Calculate bounds from loaded chunks
          let minCx = Infinity, maxCx = -Infinity;
          let minCy = Infinity, maxCy = -Infinity;
          for (const chunk of loadedChunks.values()) {
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

          console.log(`Loaded ${loadedChunks.size} chunks (connected: ${storage.isConnected})`);
        } else {
          // No chunks found, create a default chunk at origin
          chunkManager.createEmptyChunk({ cx: 0, cy: 0 });
          const newChunks = new Map<string, ReturnType<typeof chunkManager.getChunk>>();
          for (const chunk of chunkManager.getAllChunks()) {
            newChunks.set(chunkKey(chunk.coord), chunk);
          }
          setChunks(newChunks as Map<string, NonNullable<ReturnType<typeof chunkManager.getChunk>>>);
          setWorldBounds(chunkManager.getBounds());
        }

        setLoading(false);
      } catch (error) {
        console.error('Failed to initialize:', error);
        setLoading(false);
      }
    };

    init();

    // Listen for connection status changes
    const unsubscribe = storage.onConnectionChange(setConnected);
    return () => unsubscribe();
  }, [setTilesets, setEntityRegistry, setChunks, setWorldBounds, setLoading, setConnected]);

  // Auto-save every 30 seconds
  useEffect(() => {
    const autoSaveInterval = setInterval(async () => {
      const currentChunks = useEditorStore.getState().chunks;
      if (currentChunks.size > 0) {
        console.log('Auto-saving...');
        await storage.saveAllChunks(currentChunks);
      }
    }, 30000);

    return () => clearInterval(autoSaveInterval);
  }, []);

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
