<script lang="ts">
  import { DocsApp } from 'svedocs/theme';
  import Landing from '$lib/Landing.svelte';
  import components from 'virtual:svedocs/components';
  import layouts from 'virtual:svedocs/layouts';
  import themeComponents from 'virtual:svedocs/theme-components';
  import loadSearch from 'virtual:svedocs/search-loader';
  export let data;
</script>

<DocsApp page={data.page} pages={data.pages} tree={data.tree} search={data.search} config={data.config} {components} {layouts} {themeComponents} {loadSearch}>
  <svelte:fragment slot="landing" let:page let:context>
    <Landing {page} {context} />
  </svelte:fragment>
</DocsApp>
