<script lang="ts">
  import { MobileNav, ScopeSwitcher, SearchDialog, ThemeToggle, TopNav } from 'svedocs/theme';
  import type { SvedocsNavbarProps } from 'svedocs/theme/types';
  import Brand from '../Brand.svelte';

  let { context, mobileTree = [], mobileCurrentPath = '', mobileMenuId = 'vesty-menu',
    mobileMenuOpen = false, themeComponents = {}, onToggleMobileMenu, onCloseMobileMenu }: SvedocsNavbarProps = $props();
  let menuButton: HTMLButtonElement;
  const BrandComponent = $derived(themeComponents.Brand ?? Brand);
  const Nav = $derived(themeComponents.TopNav ?? TopNav);
  const Search = $derived(themeComponents.Search ?? SearchDialog);
  const Mode = $derived(themeComponents.ThemeToggle ?? ThemeToggle);
  const Mobile = $derived(themeComponents.MobileNav ?? MobileNav);
</script>

<svelte:window onclick={(event) => {
  if (event.target instanceof Element && event.target.closest('.vesty-mobile-menu a')) onCloseMobileMenu?.();
}} onkeydown={(event) => {
  if (event.key === 'Escape' && mobileMenuOpen) { onCloseMobileMenu?.(); menuButton?.focus(); }
}} />

<header class="vesty-header" class:menu-open={mobileMenuOpen} data-theme-component="navbar">
  <div class="vesty-navbar">
    <BrandComponent {context} />
    <div class="vesty-desktop-nav"><Nav {context} /></div>
    <div class="vesty-nav-tools">
      {#if context.config.search.enabled}
        <Search records={context.search} loadRecords={context.loadSearch} scope={context.searchScope}
          provider={context.config.search.provider} buildMode={context.config.build.mode} {context} />
      {/if}
      <ScopeSwitcher page={context.page} pages={context.pages} locales={context.config.i18n.locales} {context} />
      <Mode defaultMode={context.config.theme.defaultMode} {context} />
      <button bind:this={menuButton} class="vesty-menu-button" type="button" onclick={onToggleMobileMenu}
        aria-label={context.t(mobileMenuOpen ? 'nav.mobile.close' : 'nav.mobile.open')}
        aria-expanded={mobileMenuOpen} aria-controls={mobileMenuId}>
        <svg aria-hidden="true" viewBox="0 0 24 24">
          {#if mobileMenuOpen}<path d="m6 6 12 12M18 6 6 18" />{:else}<path d="M4 7h16M4 12h16M4 17h16" />{/if}
        </svg>
      </button>
    </div>
  </div>
  <div class="vesty-mobile-menu" id={mobileMenuId} hidden={!mobileMenuOpen}>
    <Nav {context} />
    <Mobile items={mobileTree} currentPath={mobileCurrentPath} {themeComponents} {context} />
  </div>
</header>
