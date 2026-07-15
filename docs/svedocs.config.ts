import { defineConfig } from 'svedocs/config';

export default defineConfig({
  site: {
    name: 'Vesty',
    title: 'Vesty documentation',
    description: 'Build VST3 effects and instruments in Rust with explicit realtime boundaries and system WebView editors.'
  },
  content: {
    root: 'content',
    docs: 'content/docs',
    pages: 'content/pages'
  },
  theme: {
    defaultMode: 'dark',
    palette: {
      accent: '#e47a5f',
      neutral: 'stone'
    },
    fonts: {
      sans: 'ui-rounded, "SF Pro Rounded", "Avenir Next", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif',
      display: 'ui-rounded, "SF Pro Rounded", "Avenir Next", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif',
      mono: 'ui-monospace, "SFMono-Regular", "Cascadia Code", Menlo, Consolas, monospace'
    },
    radius: '4px',
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
        'landing.eyebrow': 'Rust-first framework for VST3 effects and instruments',
        'landing.description': 'Author VST3 effects and instruments in Rust, keep DSP inside an explicit realtime boundary, and build the editor with a directly embedded system WebView.',
        'landing.status': 'ALPHA / VST3 FIRST',
        'landing.docs': 'Open documentation',
        'landing.github': 'View source',
        'landing.commandLabel': 'Scaffold command',
        'landing.systemLabel': 'HOST / ADAPTER / KERNEL / EDITOR',
        'landing.mapLabel': 'VESTY AUTHORING PATH',
        'landing.scope': 'realtime boundary',
        'landing.scopeValue': 'explicit + testable',
        'landing.bridge': 'parameter identity',
        'landing.bridgeValue': 'stable + typed',
        'landing.editor': 'editor runtime',
        'landing.editorValue': 'wry system WebView',
        'landing.signalTitle': 'Native DSP and Web UI, without sharing a thread.',
        'landing.signalDescription': 'The VST3 adapter borrows host audio and events into Rust process contexts. The editor talks to the controller through typed JSBridge, never to the audio kernel.',
        'landing.stageHost': 'Host',
        'landing.stageHostDescription': 'Automation, events, transport',
        'landing.stageAdapter': 'VST3 adapter',
        'landing.stageAdapterDescription': 'ABI, buses, state, ordering',
        'landing.stageKernel': 'Audio kernel',
        'landing.stageKernelDescription': 'Borrowed buffers, bounded DSP',
        'landing.stageEditor': 'Web editor',
        'landing.stageEditorDescription': 'System WebView, typed bridge',
        'landing.explore': 'Build the complete VST3 path',
        'landing.exploreDescription': 'Follow Vesty’s real workflow from stable parameters and sample-accurate processing to a packaged bundle, validator output, and DAW evidence.',
        'landing.cardStart': 'Complete plugin tutorial',
        'landing.cardStartDescription': 'Scaffold an effect, implement automation-safe DSP, test it, and package the VST3 bundle.',
        'landing.cardRealtime': 'Realtime process contract',
        'landing.cardRealtimeDescription': 'Keep allocation, locks, I/O, JSON, logging, and WebView calls outside process().',
        'landing.cardWeb': 'Host-authoritative Web UI',
        'landing.cardWebDescription': 'Initialize from ready.paramValues and send edit gestures through @vesty/plugin-ui.',
        'landing.cardShip': 'Package, validate, prove',
        'landing.cardShipDescription': 'Check bundle metadata, exports, and manifests, then collect validator and real DAW evidence.',
        'landing.contract': 'THE CONTRACT',
        'landing.contractTitle': 'No allocation. No locks. No WebView in process().',
        'landing.contractDescription': 'Vesty resolves parameter handles before processing, borrows host buffers for one block, and moves UI communication outside the audio callback.',
        'landing.contractAction': 'Review realtime rules'
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
        'landing.eyebrow': '面向 VST3 效果器与乐器的 Rust-first 框架',
        'landing.description': '使用 Rust 编写 VST3 效果器与乐器，把 DSP 保持在明确的实时边界内，并通过直接嵌入的系统 WebView 构建编辑器。',
        'landing.status': 'ALPHA / VST3 优先',
        'landing.docs': '打开文档',
        'landing.github': '查看源码',
        'landing.commandLabel': '脚手架命令',
        'landing.systemLabel': '宿主 / 适配层 / 内核 / 编辑器',
        'landing.mapLabel': 'VESTY 开发路径',
        'landing.scope': '实时边界',
        'landing.scopeValue': '显式 + 可测试',
        'landing.bridge': '参数标识',
        'landing.bridgeValue': '稳定 + 类型化',
        'landing.editor': '编辑器运行时',
        'landing.editorValue': 'wry 系统 WebView',
        'landing.signalTitle': '原生 DSP 与 Web UI，各在线程边界的一侧。',
        'landing.signalDescription': 'VST3 适配层把宿主音频与事件借用为 Rust process context；编辑器通过类型化 JSBridge 与 controller 通信，不接触音频 kernel。',
        'landing.stageHost': '宿主',
        'landing.stageHostDescription': '自动化、事件、传输',
        'landing.stageAdapter': 'VST3 适配层',
        'landing.stageAdapterDescription': 'ABI、bus、状态、排序',
        'landing.stageKernel': '音频内核',
        'landing.stageKernelDescription': '借用 buffer、有界 DSP',
        'landing.stageEditor': 'Web 编辑器',
        'landing.stageEditorDescription': '系统 WebView、类型化 bridge',
        'landing.explore': '走完整条 VST3 开发路径',
        'landing.exploreDescription': '按照 Vesty 的实际流程，从稳定参数与 sample-accurate processing 走到 bundle 打包、validator 输出和 DAW 证据。',
        'landing.cardStart': '完整插件教程',
        'landing.cardStartDescription': '创建效果器、实现可自动化 DSP、完成测试，并打包 VST3 bundle。',
        'landing.cardRealtime': '实时处理契约',
        'landing.cardRealtimeDescription': '不要在 process() 中执行分配、加锁、I/O、JSON、普通日志或 WebView 调用。',
        'landing.cardWeb': '宿主权威 Web UI',
        'landing.cardWebDescription': '使用 ready.paramValues 初始化界面，并通过 @vesty/plugin-ui 发送参数 gesture。',
        'landing.cardShip': '打包、验证、举证',
        'landing.cardShipDescription': '检查 bundle 元数据、导出和 manifest，再收集 validator 与真实 DAW 证据。',
        'landing.contract': '核心契约',
        'landing.contractTitle': 'process() 中不分配、不加锁、不调用 WebView。',
        'landing.contractDescription': 'Vesty 在处理前解析参数 handle，每个 block 只借用宿主 buffer，并把 UI 通信移出音频回调。',
        'landing.contractAction': '检查实时规则',
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
