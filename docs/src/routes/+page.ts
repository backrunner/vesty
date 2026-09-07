import { error } from '@sveltejs/kit';
import pageLoaders from 'virtual:svedocs/page-loaders';
import pages from 'virtual:svedocs/page-index';
import tree from 'virtual:svedocs/tree';
import config from 'virtual:svedocs/config';
import { svedocsPagePrerender } from 'svedocs/cloudflare';
import type { SvedocsPage } from 'svedocs/core';
import type { PageLoad } from './$types';

export const prerender = svedocsPagePrerender();

export const load: PageLoad = async () => {
  const pageIndex = pages.find((page) => page.routePath === '/');
  if (!pageIndex) error(404, 'Homepage content not found');
  const page = await loadFullPage(pageIndex);
  return { page, pages: mergeCurrentPage(pages, page), search: [], tree, config };
};

async function loadFullPage(page: SvedocsPage): Promise<SvedocsPage> {
  const loaded = await pageLoaders[page.id]?.();
  return loaded?.default ?? page;
}

function mergeCurrentPage(pages: SvedocsPage[], current: SvedocsPage): SvedocsPage[] {
  return pages.map((page) => page.id === current.id ? current : page);
}
