<script lang="ts">
  import type { Game } from '$lib/types';
  import { games } from '$lib/stores';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  let { game, onClose }: { game: Game; onClose: () => void } = $props();
  
  let editedName = $state(game.name);
  let editedExecutablePath = $state(game.executable_path || '');
  let editedCoverUrl = $state(game.cover_url || '');
  let editedCoverBase64 = $state<string | null>(null);
  let isSaving = $state(false);

  // Convert file path to web-accessible URL (base64 data URL)
  async function loadCoverAsBase64(path: string): Promise<string> {
    try {
      // Read file as bytes using Tauri fs plugin
      const bytes: number[] = await invoke('read_file_bytes', { path });
      // Determine MIME type from extension
      const ext = path.split('.').pop()?.toLowerCase();
      const mimeTypes: Record<string, string> = {
        'jpg': 'image/jpeg',
        'jpeg': 'image/jpeg',
        'png': 'image/png',
        'webp': 'image/webp',
        'gif': 'image/gif'
      };
      const mimeType = mimeTypes[ext || 'jpg'] || 'image/jpeg';
      // Convert to blob then to data URL
      const blob = new Blob([new Uint8Array(bytes)], { type: mimeType });
      return new Promise((resolve) => {
        const reader = new FileReader();
        reader.onloadend = () => resolve(reader.result as string);
        reader.readAsDataURL(blob);
      });
    } catch (error) {
      console.error('Failed to load image:', error);
      return path;
    }
  }

  // Get display URL for cover image
  function getCoverDisplayUrl(path: string): string {
    if (!path) return '';
    // If it's already a data URL, return as-is
    if (path.startsWith('data:')) {
      return path;
    }
    // If it's http/https, return as-is
    if (path.startsWith('http://') || path.startsWith('https://')) {
      return path;
    }
    // For local paths, we'll load them as base64
    return path;
  }

  async function handleBrowseExecutable() {
    try {
      const filePath = await open({
        title: 'Select Game Executable',
        multiple: false,
        filters: [{
          name: 'All Files',
          extensions: ['*']
        }]
      });
      
      if (filePath) {
        editedExecutablePath = filePath as string;
      }
    } catch (error) {
      console.error('Failed to browse for executable:', error);
    }
  }

  async function handleBrowseCover() {
    try {
      const filePath = await open({
        title: 'Select Cover Image',
        multiple: false,
        filters: [{
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'webp']
        }]
      });
      
      if (filePath) {
        editedCoverUrl = filePath as string;
        // Load image as base64 for preview
        editedCoverBase64 = await loadCoverAsBase64(filePath as string);
      }
    } catch (error) {
      console.error('Failed to browse for cover image:', error);
    }
  }

  async function handleSave() {
    if (!editedName.trim()) {
      alert('Game name cannot be empty');
      return;
    }
    
    if (!editedExecutablePath.trim()) {
      alert('Executable path cannot be empty');
      return;
    }

    isSaving = true;
    
    try {
      const { invoke } = await import('@tauri-apps/api/core');

      // Store base64 data URL if available, otherwise store the path
      const coverToStore = editedCoverBase64 || editedCoverUrl;
      
      console.log('Saving cover URL length:', coverToStore?.length || 0);
      console.log('Cover URL starts with:', coverToStore?.substring(0, 50) || 'null');

      await invoke('update_custom_game', {
        gameId: game.id,
        name: editedName.trim(),
        executablePath: editedExecutablePath.trim(),
        coverUrl: coverToStore?.trim() || null
      });

      // Update the specific game in the store immediately
      const coverUrl = coverToStore?.trim() || null;
      games.updateGame(game.id, {
        name: editedName.trim(),
        executable_path: editedExecutablePath.trim(),
        cover_url: coverUrl
      });

      // Close modal
      onClose();
      
      // Refresh the entire app after 1 second to show updated cover
      setTimeout(() => {
        window.location.reload();
      }, 1000);
    } catch (error) {
      console.error('Failed to update game:', error);
      alert('Failed to update game: ' + error);
    } finally {
      isSaving = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  // Trap focus in modal
  $effect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div class="modal-overlay" onclick={onClose}>
  <div class="modal-content" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2 class="text-mono text-lg text-primary">Edit Game</h2>
      <button class="close-button" onclick={onClose}>×</button>
    </div>
    
    <div class="modal-body">
      <div class="form-group">
        <label class="text-mono text-sm text-secondary">Game Name</label>
        <input 
          type="text" 
          bind:value={editedName}
          class="form-input"
          placeholder="Enter game name"
        />
      </div>
      
      <div class="form-group">
        <label class="text-mono text-sm text-secondary">Executable Path</label>
        <div class="input-with-button">
          <input 
            type="text" 
            bind:value={editedExecutablePath}
            class="form-input"
            placeholder="Path to executable"
          />
          <button class="browse-button" onclick={handleBrowseExecutable}>
            Browse
          </button>
        </div>
      </div>
      
      <div class="form-group">
        <label class="text-mono text-sm text-secondary">Cover Image Path</label>
        <div class="input-with-button">
          <input 
            type="text" 
            bind:value={editedCoverUrl}
            class="form-input"
            placeholder="Path to cover image (optional)"
          />
          <button class="browse-button" onclick={handleBrowseCover}>
            Browse
          </button>
        </div>
      </div>
      
      {#if editedCoverUrl}
        <div class="cover-preview">
          <label class="text-mono text-sm text-secondary">Preview</label>
          <img src={editedCoverBase64 || getCoverDisplayUrl(editedCoverUrl)} alt="Cover preview" class="preview-image" />
        </div>
      {/if}
    </div>
    
    <div class="modal-footer">
      <button class="cancel-button" onclick={onClose} disabled={isSaving}>
        Cancel
      </button>
      <button class="save-button" onclick={handleSave} disabled={isSaving}>
        {#if isSaving}
          Saving...
        {:else}
          Save Changes
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }
  
  .modal-content {
    background: var(--color-bg-2);
    border: 1px solid var(--color-border-1);
    border-radius: var(--radius-lg);
    width: 100%;
    max-width: 500px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
  
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border-1);
  }
  
  .close-button {
    background: transparent;
    border: none;
    color: var(--color-text-secondary);
    font-size: 24px;
    cursor: pointer;
    padding: var(--space-2);
    line-height: 1;
  }
  
  .close-button:hover {
    color: var(--color-text-primary);
  }
  
  .modal-body {
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  
  .form-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  
  .form-input {
    background: var(--color-bg-1);
    border: 1px solid var(--color-border-1);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    font-size: var(--text-md);
    color: var(--color-text-primary);
    font-family: var(--font-sans);
  }
  
  .form-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }
  
  .input-with-button {
    display: flex;
    gap: var(--space-2);
  }
  
  .input-with-button .form-input {
    flex: 1;
  }
  
  .browse-button {
    background: var(--color-bg-3);
    border: 1px solid var(--color-border-1);
    color: var(--color-text-primary);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    cursor: pointer;
    white-space: nowrap;
  }
  
  .browse-button:hover {
    background: var(--color-bg-2);
    border-color: var(--color-accent);
  }
  
  .cover-preview {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  
  .preview-image {
    width: 100%;
    max-width: 200px;
    aspect-ratio: 3/4;
    object-fit: cover;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border-1);
  }
  
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    padding: var(--space-4);
    border-top: 1px solid var(--color-border-1);
  }
  
  .cancel-button,
  .save-button {
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }
  
  .cancel-button {
    background: var(--color-bg-2);
    border: 1px solid var(--color-border-1);
    color: var(--color-text-primary);
  }
  
  .cancel-button:hover {
    background: var(--color-bg-3);
  }
  
  .save-button {
    background: var(--color-accent);
    border: 1px solid var(--color-accent);
    color: var(--color-text-primary);
  }
  
  .save-button:hover {
    background: #f03030;
  }
  
  .cancel-button:disabled,
  .save-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
