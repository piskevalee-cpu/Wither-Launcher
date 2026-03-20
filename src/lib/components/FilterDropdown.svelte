<script lang="ts">
  import { onMount } from 'svelte'
  import { activeFilters } from '$lib/stores'

  const OPTIONS = [
    { key: 'owned',     label: 'Owned' },
    { key: 'installed', label: 'Installed' },
    { key: 'custom',    label: 'Custom' },
    { key: 'recent',    label: 'Recently played' },
    { key: 'favourite', label: 'Favourites' },
  ]

  let open = false

  function toggle(key: string) {
    activeFilters.update(filters => {
      if (filters.includes(key)) {
        return filters.filter(f => f !== key)
      } else {
        return [...filters, key]
      }
    })
  }

  function handleOutsideClick(e: MouseEvent) {
    if (!(e.target as Element).closest('.filter-wrapper')) open = false
  }

  onMount(() => {
    document.addEventListener('click', handleOutsideClick)
    return () => document.removeEventListener('click', handleOutsideClick)
  })
</script>

<div class="filter-wrapper" style="position: relative;">
  <button class="tb-btn" class:active={$activeFilters.length > 0} onclick={() => open = !open}>
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.3">
      <path d="M1 3h10M3 6h6M5 9h2"/>
    </svg>
    Filter{$activeFilters.length > 0 ? ` · ${$activeFilters.length}` : ''}
  </button>

  {#if open}
    <div class="filter-dropdown">
      {#each OPTIONS as opt}
        <div
          class="filter-option"
          class:active={$activeFilters.includes(opt.key)}
          onclick={() => toggle(opt.key)}
        >
          <div class="filter-checkbox">
            {#if $activeFilters.includes(opt.key)}
              <svg width="9" height="9" viewBox="0 0 9 9" fill="none" stroke="#000" stroke-width="1.6">
                <path d="M1.5 4.5l2 2 4-4" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            {/if}
          </div>
          {opt.label}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tb-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--text-base);
    font-weight: 400;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tb-btn:hover {
    background: var(--bg-s2);
  }

  .tb-btn.active {
    background: var(--bg-s3);
    border-color: var(--border-2);
    color: var(--text-primary);
  }

  .filter-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    background: rgba(18, 18, 18, 0.98);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 12px;
    padding: 8px;
    min-width: 180px;
    z-index: 1000;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.8);
    animation: slideIn 0.15s ease-out;
  }

  .filter-option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 7px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 400;
    color: rgba(255, 255, 255, 0.5);
    transition: background 0.12s, color 0.12s;
  }

  .filter-option:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #fff;
  }

  .filter-option.active {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .filter-checkbox {
    width: 14px;
    height: 14px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.12s, border-color 0.12s;
  }

  .filter-option.active .filter-checkbox {
    background: #ffffff;
    border-color: #ffffff;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
