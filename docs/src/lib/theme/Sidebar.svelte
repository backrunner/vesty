<script lang="ts">
  import { SidebarTree } from 'svedocs/theme';
  import { resolveLocalizedHref, useSvedocsTheme } from 'svedocs/theme/headless';
  import type { SvedocsSidebarProps } from 'svedocs/theme/types';

  let { items = [], currentPath = '', depth = 0 }: SvedocsSidebarProps = $props();
  const context = useSvedocsTheme();
</script>

<div class="vesty-sidebar-tree">
  {#if depth === 0}
    <div class="vesty-sidebar-heading"><span>{$context.t('nav.handbook')}</span><span>α</span></div>
  {/if}
  <SidebarTree {items} {currentPath} {depth} />
  {#if depth === 0}
    <a class="vesty-sidebar-start" href={resolveLocalizedHref('/docs/quick-start', $context)}>
      <span aria-hidden="true">❯</span>{$context.t('home.primaryAction')}<span aria-hidden="true">↗</span>
    </a>
  {/if}
</div>
