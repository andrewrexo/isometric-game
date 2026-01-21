# Mapper Server Design

## Overview

Add a backend server to the mapper so map data persists on the server and multiple people can edit (turn-based, last-save-wins).

## Requirements

- File-based storage (chunks saved as JSON files)
- Open access (no authentication)
- Turn-based editing (no conflict resolution)
- Hosted on same Hetzner VPS as frontend
- Auto-save every 30 seconds + debounced save on edit

## API Endpoints

```
GET    /api/chunks           - List all chunk coordinates
GET    /api/chunks/:cx/:cy   - Get a single chunk
PUT    /api/chunks/:cx/:cy   - Save a single chunk
DELETE /api/chunks/:cx/:cy   - Delete a chunk
GET    /api/map/export       - Download entire map as JSON
POST   /api/map/import       - Upload entire map JSON
```

## File Storage Structure

```
mapper/
  server/
    index.ts              - Express server
    package.json          - Server dependencies
  src/                    - Frontend (existing)
  mapper-data/            - Runtime data directory
    chunks/
      0_0.json
      0_1.json
      ...
```

## Frontend Changes

### Storage Layer

- New API client in `Storage.ts` that talks to server
- On startup: fetch all chunks from server
- On edit: debounced save to server (500ms delay)
- Auto-save: periodic save every 30 seconds
- Offline fallback: use IndexedDB if server unreachable, show "disconnected" indicator

### Migration

- Add export button to download current IndexedDB data
- Import endpoint to restore data on server

## Deployment

- Express serves both static frontend (`dist/`) and API (`/api`)
- Single process on single port (e.g., 3000)
- nginx proxies to Express (or Express serves directly)
- `mapper-data/` folder persists map data - should be backed up

## Implementation Steps

1. Create server with Express + file-based chunk storage
2. Add API client to frontend Storage module
3. Update store to use server storage with IndexedDB fallback
4. Add auto-save interval (30s)
5. Add connection status indicator
6. Add export/import UI for migration
7. Update deployment config
