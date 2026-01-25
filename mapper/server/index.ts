import express from 'express';
import cors from 'cors';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const app = express();
const PORT = process.env.PORT || 3000;

// Detect if running from compiled dist or source
// In dev: __dirname = /path/to/mapper/server
// In prod: __dirname = /path/to/mapper/server/dist
const isCompiled = __dirname.endsWith('dist');
const serverRoot = isCompiled ? path.join(__dirname, '..') : __dirname;
const mapperRoot = path.join(serverRoot, '..');

// Data directory for chunk storage
const DATA_DIR = path.join(mapperRoot, 'mapper-data');
const CHUNKS_DIR = path.join(DATA_DIR, 'chunks');
const INTERIORS_DIR = path.join(DATA_DIR, 'interiors');

// Game server maps directory (for deploy)
const GAME_SERVER_DIR = path.join(mapperRoot, '..', 'rust-server', 'maps');
const GAME_CHUNKS_DIR = path.join(GAME_SERVER_DIR, 'world_0');
const GAME_INTERIORS_DIR = path.join(GAME_SERVER_DIR, 'interiors');

// Ensure data directories exist
async function ensureDataDirs() {
  await fs.mkdir(CHUNKS_DIR, { recursive: true });
  await fs.mkdir(INTERIORS_DIR, { recursive: true });
}

// Middleware
app.use(cors());
app.use(express.json({ limit: '50mb' }));

// Frontend dist path (served after API routes)
const distPath = path.join(mapperRoot, 'dist');

// Serve mapper-config.json from root
app.get('/mapper-config.json', (_req, res) => {
  res.sendFile(path.join(mapperRoot, 'mapper-config.json'));
});

// --- Chunk API ---

// List all chunks
app.get('/api/chunks', async (_req, res) => {
  try {
    const files = await fs.readdir(CHUNKS_DIR);
    const chunks = files
      .filter(f => f.endsWith('.json'))
      .map(f => {
        const [cx, cy] = f.replace('.json', '').split('_').map(Number);
        return { cx, cy };
      });
    res.json(chunks);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.json([]);
    } else {
      console.error('Error listing chunks:', err);
      res.status(500).json({ error: 'Failed to list chunks' });
    }
  }
});

// Get all chunks in one request
app.get('/api/chunks/all', async (_req, res) => {
  try {
    const files = await fs.readdir(CHUNKS_DIR);
    const chunks: Record<string, unknown> = {};

    for (const file of files) {
      if (!file.endsWith('.json')) continue;
      const filePath = path.join(CHUNKS_DIR, file);
      const data = await fs.readFile(filePath, 'utf-8');
      const key = file.replace('.json', '').replace('_', ',');
      chunks[key] = JSON.parse(data);
    }

    res.json(chunks);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.json({});
    } else {
      console.error('Error loading all chunks:', err);
      res.status(500).json({ error: 'Failed to load chunks' });
    }
  }
});

// Get single chunk
app.get('/api/chunks/:cx/:cy', async (req, res) => {
  try {
    const { cx, cy } = req.params;
    const filePath = path.join(CHUNKS_DIR, `${cx}_${cy}.json`);
    const data = await fs.readFile(filePath, 'utf-8');
    res.json(JSON.parse(data));
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.status(404).json({ error: 'Chunk not found' });
    } else {
      console.error('Error reading chunk:', err);
      res.status(500).json({ error: 'Failed to read chunk' });
    }
  }
});

// Save single chunk
app.put('/api/chunks/:cx/:cy', async (req, res) => {
  try {
    const { cx, cy } = req.params;
    const chunk = req.body;
    const filePath = path.join(CHUNKS_DIR, `${cx}_${cy}.json`);
    await fs.writeFile(filePath, JSON.stringify(chunk, null, 2));
    res.json({ success: true });
  } catch (err) {
    console.error('Error saving chunk:', err);
    res.status(500).json({ error: 'Failed to save chunk' });
  }
});

// Save multiple chunks at once
app.put('/api/chunks', async (req, res) => {
  try {
    const chunks = req.body as Record<string, unknown>;

    for (const [key, chunk] of Object.entries(chunks)) {
      const [cx, cy] = key.split(',');
      const filePath = path.join(CHUNKS_DIR, `${cx}_${cy}.json`);
      await fs.writeFile(filePath, JSON.stringify(chunk, null, 2));
    }

    res.json({ success: true, saved: Object.keys(chunks).length });
  } catch (err) {
    console.error('Error saving chunks:', err);
    res.status(500).json({ error: 'Failed to save chunks' });
  }
});

// Delete chunk
app.delete('/api/chunks/:cx/:cy', async (req, res) => {
  try {
    const { cx, cy } = req.params;
    const filePath = path.join(CHUNKS_DIR, `${cx}_${cy}.json`);
    await fs.unlink(filePath);
    res.json({ success: true });
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.json({ success: true }); // Already gone
    } else {
      console.error('Error deleting chunk:', err);
      res.status(500).json({ error: 'Failed to delete chunk' });
    }
  }
});

// --- Interior Maps API ---

// List all interior maps
app.get('/api/interiors', async (_req, res) => {
  try {
    const files = await fs.readdir(INTERIORS_DIR);
    const interiors = files
      .filter(f => f.endsWith('.json'))
      .map(f => f.replace('.json', ''));
    res.json({ interiors });
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.json({ interiors: [] });
    } else {
      console.error('Error listing interiors:', err);
      res.status(500).json({ error: 'Failed to list interiors' });
    }
  }
});

// Get single interior map
app.get('/api/interiors/:id', async (req, res) => {
  try {
    const { id } = req.params;
    const filePath = path.join(INTERIORS_DIR, `${id}.json`);
    const data = await fs.readFile(filePath, 'utf-8');
    res.json(JSON.parse(data));
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.status(404).json({ error: 'Interior not found' });
    } else {
      console.error('Error reading interior:', err);
      res.status(500).json({ error: 'Failed to read interior' });
    }
  }
});

// Save interior map
app.put('/api/interiors/:id', async (req, res) => {
  try {
    const { id } = req.params;
    const interior = req.body;
    const filePath = path.join(INTERIORS_DIR, `${id}.json`);
    await fs.writeFile(filePath, JSON.stringify(interior, null, 2));
    res.json({ success: true });
  } catch (err) {
    console.error('Error saving interior:', err);
    res.status(500).json({ error: 'Failed to save interior' });
  }
});

// Delete interior map
app.delete('/api/interiors/:id', async (req, res) => {
  try {
    const { id } = req.params;
    const filePath = path.join(INTERIORS_DIR, `${id}.json`);
    await fs.unlink(filePath);
    res.json({ success: true });
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      res.json({ success: true }); // Already gone
    } else {
      console.error('Error deleting interior:', err);
      res.status(500).json({ error: 'Failed to delete interior' });
    }
  }
});

// --- Map Export/Import ---

// Export entire map
app.get('/api/map/export', async (_req, res) => {
  try {
    const files = await fs.readdir(CHUNKS_DIR);
    const chunks: Record<string, unknown> = {};

    for (const file of files) {
      if (!file.endsWith('.json')) continue;
      const filePath = path.join(CHUNKS_DIR, file);
      const data = await fs.readFile(filePath, 'utf-8');
      const key = file.replace('.json', '').replace('_', ',');
      chunks[key] = JSON.parse(data);
    }

    res.setHeader('Content-Disposition', 'attachment; filename="map-export.json"');
    res.json({
      version: 1,
      exportedAt: new Date().toISOString(),
      chunks
    });
  } catch (err) {
    console.error('Error exporting map:', err);
    res.status(500).json({ error: 'Failed to export map' });
  }
});

// Import entire map
app.post('/api/map/import', async (req, res) => {
  try {
    const { chunks } = req.body;

    if (!chunks || typeof chunks !== 'object') {
      return res.status(400).json({ error: 'Invalid import format' });
    }

    // Clear existing chunks
    try {
      const existingFiles = await fs.readdir(CHUNKS_DIR);
      for (const file of existingFiles) {
        await fs.unlink(path.join(CHUNKS_DIR, file));
      }
    } catch {
      // Directory might not exist yet
    }

    // Write new chunks
    let count = 0;
    for (const [key, chunk] of Object.entries(chunks)) {
      const [cx, cy] = key.split(',');
      const filePath = path.join(CHUNKS_DIR, `${cx}_${cy}.json`);
      await fs.writeFile(filePath, JSON.stringify(chunk, null, 2));
      count++;
    }

    res.json({ success: true, imported: count });
  } catch (err) {
    console.error('Error importing map:', err);
    res.status(500).json({ error: 'Failed to import map' });
  }
});

// --- Deploy to Game Server ---

// Convert collision array to base64-encoded bitset
function collisionToBase64(collision: number[], size: number): string {
  const byteLength = Math.ceil(size / 8);
  const bytes = new Uint8Array(byteLength);

  for (let i = 0; i < collision.length && i < byteLength; i++) {
    bytes[i] = collision[i];
  }

  // Convert to base64
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return Buffer.from(binary, 'binary').toString('base64');
}

// Convert mapper chunk format to game server format
function convertChunkToGameFormat(mapperChunk: Record<string, unknown>): Record<string, unknown> {
  const width = mapperChunk.width as number;
  const height = mapperChunk.height as number;
  const size = width; // Assuming square chunks

  // Convert collision array to base64
  const collisionArray = mapperChunk.collision as number[] || [];
  const collisionBase64 = collisionToBase64(collisionArray, size * size);

  return {
    version: 2,
    coord: mapperChunk.coord,
    size,
    layers: mapperChunk.layers,
    collision: collisionBase64,
    entities: mapperChunk.entities || [],
    mapObjects: mapperChunk.mapObjects || [],
    walls: mapperChunk.walls || [],
    portals: mapperChunk.portals || [],
  };
}

// Deploy maps to game server directory
app.post('/api/deploy', async (_req, res) => {
  try {
    // Ensure game server directories exist
    await fs.mkdir(GAME_CHUNKS_DIR, { recursive: true });
    await fs.mkdir(GAME_INTERIORS_DIR, { recursive: true });

    let chunksCopied = 0;
    let interiorsCopied = 0;

    // Convert and copy chunks (mapper format -> game server format)
    try {
      const chunkFiles = await fs.readdir(CHUNKS_DIR);
      for (const file of chunkFiles) {
        if (!file.endsWith('.json')) continue;
        const srcPath = path.join(CHUNKS_DIR, file);

        // Read and convert chunk
        const data = await fs.readFile(srcPath, 'utf-8');
        const mapperChunk = JSON.parse(data);
        const gameChunk = convertChunkToGameFormat(mapperChunk);

        // Write to game server with chunk_ prefix
        const destFilename = `chunk_${file}`;
        const destPath = path.join(GAME_CHUNKS_DIR, destFilename);
        await fs.writeFile(destPath, JSON.stringify(gameChunk, null, 2));
        chunksCopied++;
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== 'ENOENT') {
        throw err;
      }
    }

    // Copy interiors (same filename format - already in correct format)
    try {
      const interiorFiles = await fs.readdir(INTERIORS_DIR);
      for (const file of interiorFiles) {
        if (!file.endsWith('.json')) continue;
        const srcPath = path.join(INTERIORS_DIR, file);
        const destPath = path.join(GAME_INTERIORS_DIR, file);
        await fs.copyFile(srcPath, destPath);
        interiorsCopied++;
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== 'ENOENT') {
        throw err;
      }
    }

    console.log(`Deployed ${chunksCopied} chunks and ${interiorsCopied} interiors to game server`);
    res.json({
      success: true,
      chunksCopied,
      interiorsCopied,
      destination: GAME_SERVER_DIR
    });
  } catch (err) {
    console.error('Deploy failed:', err);
    res.status(500).json({ error: `Deploy failed: ${(err as Error).message}` });
  }
});

// Serve static frontend files (after API routes)
app.use(express.static(distPath));

// SPA fallback - serve index.html for non-API routes
app.get('*', (req, res) => {
  // Don't serve HTML for API routes
  if (req.path.startsWith('/api')) {
    return res.status(404).json({ error: 'Not found' });
  }
  res.sendFile(path.join(distPath, 'index.html'));
});

// Start server
async function main() {
  await ensureDataDirs();
  console.log('Paths:');
  console.log('  mapperRoot:', mapperRoot);
  console.log('  distPath:', distPath);
  console.log('  dataDir:', DATA_DIR);
  app.listen(PORT, () => {
    console.log(`Mapper server running on http://localhost:${PORT}`);
  });
}

main().catch(console.error);
