<script lang="ts">
  import { linkRel, resolveLocalizedNavItem } from 'svedocs/theme/headless';
  import type { SvedocsThemeContext } from 'svedocs/theme/types';

  export let context: SvedocsThemeContext;
  $: footer = context.config.theme.footer;
</script>

{#if footer !== false && footer.links.length > 0}
  <nav aria-label={context.t('nav.footer')} data-theme-component="footer-links">
    {#each footer.links as item}
      {@const link = resolveLocalizedNavItem(item, context)}
      <a href={link.href} rel={linkRel(link)} target={link.external ? '_blank' : undefined}>
        {link.label}
      </a>
    {/each}
  </nav>
{/if}
