import { defineConfig } from 'svedocs/config';

export default defineConfig({
  site: {
    name: 'Vesty',
    title: 'Vesty documentation',
    description: 'Build realtime-safe VST3 plugins in Rust with system WebView interfaces.'
  },
  content: {
    root: 'content',
    docs: 'content/docs',
    pages: 'content/pages'
  },
  theme: {
    defaultMode: 'dark',
    palette: {
      accent: '#62e6a7',
      neutral: 'zinc'
    },
    fonts: {
      sans: '"Avenir Next", Avenir, "Segoe UI", sans-serif',
      display: '"DIN Alternate", "Avenir Next Condensed", "Avenir Next", sans-serif',
      mono: '"IBM Plex Mono", Menlo, Monaco, monospace'
    },
    radius: '2px',
    codeTheme: {
      light: 'github-light',
      dark: 'vesper'
    },
    brand: {
      label: 'VESTY',
      href: '/',
      logo: '/favicon.svg'
    },
    nav: [
      { label: 'Docs', labelKey: 'nav.docs', href: '/docs' },
      { label: 'Concepts', labelKey: 'nav.concepts', href: '/docs/concepts' },
      { label: 'Guides', labelKey: 'nav.guides', href: '/docs/guides' },
      { label: 'Reference', labelKey: 'nav.reference', href: '/docs/reference' }
    ],
    social: [
      { label: 'GitHub', href: 'https://github.com/orchiliao/vesty', external: true }
    ],
    footer: {
      text: 'Rust in the audio thread. Web tools in the editor.',
      links: [
        { label: 'Apache-2.0', href: 'https://github.com/orchiliao/vesty/blob/main/LICENSE-APACHE', external: true },
        { label: 'GitHub', href: 'https://github.com/orchiliao/vesty', external: true }
      ]
    },
    home: {
      primaryAction: { label: 'Start building', labelKey: 'home.primaryAction', href: '/docs' },
      secondaryAction: { label: 'Read the architecture', labelKey: 'home.secondaryAction', href: '/docs/concepts/architecture' }
    }
  },
  search: {
    enabled: true,
    provider: 'local',
    scope: 'current'
  },
  ai: false,
  i18n: {
    defaultLocale: 'en',
    locales: [
      { code: 'en', label: 'English', hreflang: 'en', dir: 'ltr' },
      { code: 'zh', label: '简体中文', hreflang: 'zh-CN', dir: 'ltr' }
    ],
    messages: {
      en: {
        'nav.docs': 'Docs',
        'nav.concepts': 'Concepts',
        'nav.guides': 'Guides',
        'nav.reference': 'Reference',
        'home.primaryAction': 'Start building',
        'home.secondaryAction': 'Read the architecture',
        'landing.eyebrow': 'Rust-first VST3 framework',
        'landing.description': 'Keep DSP deterministic and native. Build the editor with the web stack your team already knows.',
        'landing.status': 'ALPHA / VST3 FIRST',
        'landing.docs': 'Open documentation',
        'landing.github': 'View source',
        'landing.scope': 'realtime scope',
        'landing.scopeValue': '0 allocations',
        'landing.bridge': 'bridge protocol',
        'landing.bridgeValue': 'typed + versioned',
        'landing.editor': 'editor runtime',
        'landing.editorValue': 'system WebView',
        'landing.signalTitle': 'One boundary. Explicit ownership.',
        'landing.signalDescription': 'Audio never waits for the UI. Parameters cross the boundary through typed, testable contracts.',
        'landing.stageHost': 'Host',
        'landing.stageHostDescription': 'Automation, transport, state',
        'landing.stageAdapter': 'VST3 adapter',
        'landing.stageAdapterDescription': 'ABI guard, event ordering',
        'landing.stageKernel': 'Audio kernel',
        'landing.stageKernelDescription': 'Fixed work, realtime safe',
        'landing.stageEditor': 'Web editor',
        'landing.stageEditorDescription': 'Typed JSBridge, current state',
        'landing.explore': 'Explore the system',
        'landing.exploreDescription': 'Move from a compiling plugin to packaging and release evidence without guessing at the boundaries.',
        'landing.cardStart': 'Quick start',
        'landing.cardStartDescription': 'Create a plugin, declare parameters, and process the first audio block.',
        'landing.cardRealtime': 'Realtime contract',
        'landing.cardRealtimeDescription': 'Understand the operations that are safe inside process().',
        'landing.cardWeb': 'Web UI bridge',
        'landing.cardWebDescription': 'Connect React, Vue, Svelte, or vanilla UI to host-authoritative parameters.',
        'landing.cardShip': 'Package and validate',
        'landing.cardShipDescription': 'Produce a VST3 bundle and collect honest release evidence.',
        'landing.contract': 'THE CONTRACT',
        'landing.contractTitle': 'No locks. No allocation. No ambiguity.',
        'landing.contractDescription': 'Vesty makes the realtime boundary visible in the API, then tests the behavior on both sides of it.',
        'landing.contractAction': 'Read realtime safety'
      },
      zh: {
        'nav.primary': '主导航',
        'nav.docs': '文档',
        'nav.concepts': '核心概念',
        'nav.guides': '开发指南',
        'nav.reference': '参考',
        'nav.documentation': '文档导航',
        'nav.footer': '页脚',
        'nav.social': '社交链接',
        'nav.mobile.open': '打开菜单',
        'nav.mobile.close': '关闭菜单',
        'nav.skipToContent': '跳到正文',
        'search.trigger': '搜索',
        'search.dialog': '搜索文档',
        'search.query': '搜索关键词',
        'search.placeholder': '搜索 Vesty 文档',
        'search.results': '搜索结果',
        'search.loading': '正在搜索...',
        'search.loadingIndex': '正在加载搜索索引...',
        'search.indexError': '无法加载搜索索引。',
        'search.empty': '没有匹配的文档。',
        'toc.label': '本页内容',
        'article.kind.doc': '文档',
        'article.kind.page': '页面',
        'article.breadcrumb': '面包屑',
        'article.updated': '更新于 {date}',
        'article.edit': '编辑此页',
        'article.previous': '上一页',
        'article.next': '下一页',
        'heading.anchor': '链接到此章节',
        'code.copy': '复制代码',
        'code.copied': '已复制',
        'tools.label': '页面工具',
        'tools.backToTop': '回到顶部',
        'theme.switch': '切换到{mode}主题',
        'theme.light': '浅色',
        'theme.dark': '深色',
        'home.primaryAction': '开始构建',
        'home.secondaryAction': '阅读架构',
        'landing.eyebrow': 'Rust-first VST3 框架',
        'landing.description': '让 DSP 保持原生、确定且实时安全；编辑器继续使用团队熟悉的 Web 技术栈。',
        'landing.status': 'ALPHA / VST3 优先',
        'landing.docs': '打开文档',
        'landing.github': '查看源码',
        'landing.scope': '实时作用域',
        'landing.scopeValue': '零分配',
        'landing.bridge': '桥接协议',
        'landing.bridgeValue': '类型化 + 版本化',
        'landing.editor': '编辑器运行时',
        'landing.editorValue': '系统 WebView',
        'landing.signalTitle': '一道边界，所有权清晰。',
        'landing.signalDescription': '音频线程永远不等待 UI，参数通过类型化、可测试的契约跨越边界。',
        'landing.stageHost': '宿主',
        'landing.stageHostDescription': '自动化、传输、状态',
        'landing.stageAdapter': 'VST3 适配层',
        'landing.stageAdapterDescription': 'ABI 防护、事件排序',
        'landing.stageKernel': '音频内核',
        'landing.stageKernelDescription': '固定工作量、实时安全',
        'landing.stageEditor': 'Web 编辑器',
        'landing.stageEditorDescription': '类型化 JSBridge、当前状态',
        'landing.explore': '探索完整系统',
        'landing.exploreDescription': '从第一个可编译插件走到打包和发布证据，每一步都有明确边界。',
        'landing.cardStart': '快速开始',
        'landing.cardStartDescription': '创建插件、声明参数并处理第一个音频块。',
        'landing.cardRealtime': '实时契约',
        'landing.cardRealtimeDescription': '理解 process() 中允许和禁止的操作。',
        'landing.cardWeb': 'Web UI 桥接',
        'landing.cardWebDescription': '将 React、Vue、Svelte 或原生 UI 连接到宿主权威参数。',
        'landing.cardShip': '打包与验证',
        'landing.cardShipDescription': '生成 VST3 bundle，并收集真实可信的发布证据。',
        'landing.contract': '核心契约',
        'landing.contractTitle': '无锁、无分配、无歧义。',
        'landing.contractDescription': 'Vesty 在 API 中暴露实时边界，并在边界两侧验证行为。',
        'landing.contractAction': '阅读实时安全',
        'error.notFound.title': '页面未找到',
        'error.notFound.description': '这个文档集中没有你正在查找的页面。',
        'error.generic.title': '出现了问题',
        'error.generic.description': '页面恢复期间，文档导航仍然可用。',
        'error.status': '错误 {status}',
        'error.home': '首页',
        'error.docs': '文档'
      }
    }
  },
  source: {
    editBaseUrl: 'https://github.com/orchiliao/vesty/edit/main/docs'
  },
  checks: {
    assets: true,
    externalLinks: false,
    translations: true
  },
  seo: {
    sitemap: true,
    robots: true,
    defaultAuthor: 'Vesty contributors',
    ogImage: {
      template: 'default',
      format: 'svg',
      outDir: 'static/og',
      renderer: 'svg'
    }
  }
});
