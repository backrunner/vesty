<script lang="ts">
  import { resolveLocalizedHref } from 'svedocs/theme/headless';
  import type { SvedocsThemeContext } from 'svedocs/theme/types';

  export let context: SvedocsThemeContext;
</script>

<a class="sd-brand vesty-brand" href={resolveLocalizedHref('/', context)} aria-label="Vesty" data-theme-component="brand">
  <img class="vesty-brand-symbol" src="/brand/vesty-mark.svg" alt="" width="30" height="30" />
  <img class="vesty-brand-wordmark" src="/brand/vesty-wordmark.svg" alt="" width="78" height="28" />
</a>
