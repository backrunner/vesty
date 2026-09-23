<script lang="ts">
  import { resolveLocalizedHref } from 'svedocs/theme/headless';
  import type { SvedocsFooterProps } from 'svedocs/theme/types';
  import Brand from '../Brand.svelte';
  import FooterLinks from '../FooterLinks.svelte';

  let { context, themeComponents = {} }: SvedocsFooterProps = $props();
  const Links = $derived(themeComponents.FooterLinks ?? FooterLinks);
</script>

{#if context.config.theme.footer !== false}
  <footer class="vesty-footer" data-theme-component="footer">
    <div class="vesty-footer-main">
      <div><Brand {context} /><p>{context.t('footer.text')}</p></div>
      <div class="vesty-footer-links">
        <a href={resolveLocalizedHref('/docs/quick-start', context)}>{context.t('home.primaryAction')} <span aria-hidden="true">↗</span></a>
        <Links {context} />
      </div>
    </div>
    <div class="vesty-footer-meta"><span>VESTY / RUST + WEB</span><a href={resolveLocalizedHref('/docs/tooling/release-evidence', context)}>{context.t('landing.releaseEvidence')} <span aria-hidden="true">→</span></a></div>
  </footer>
{/if}
