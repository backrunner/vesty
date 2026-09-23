import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { svedocs } from 'svedocs/vite';
import svedocsConfig from './svedocs.config.ts';

export default defineConfig({
  plugins: [
    svedocs({
      config: svedocsConfig,
      theme: { components: {
        Brand: '$lib/Brand.svelte',
        Navbar: '$lib/theme/Navbar.svelte',
        Sidebar: '$lib/theme/Sidebar.svelte',
        Footer: '$lib/theme/Footer.svelte',
        FooterLinks: '$lib/FooterLinks.svelte'
      } }
    }),
    tailwindcss(),
    sveltekit()
  ]
});
