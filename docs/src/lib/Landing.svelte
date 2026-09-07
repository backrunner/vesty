<script lang="ts">
  import type { SvedocsPage } from 'svedocs/core';
  import type { SvedocsThemeContext } from 'svedocs/theme/types';
  import { resolveLocalizedHref } from 'svedocs/theme/headless';

  export let page: SvedocsPage;
  export let context: SvedocsThemeContext;

  let template = 'gain';
  let copyState: 'idle' | 'copied' | 'failed' = 'idle';
  $: command = `vesty new my-plugin --template ${template} \\\n  --vesty-path "$VESTY_SOURCE/crates/vesty"${template === 'gain' ? '' : ' \\\n  --plugin-ui-path "$VESTY_SOURCE/packages/plugin-ui"'}`;

  function selectTemplate(value: string) {
    template = value;
    copyState = 'idle';
  }

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(command);
      copyState = 'copied';
    } catch {
      copyState = 'failed';
    }
  }

  const guides = [
    ['01', 'landing.cardStart', 'landing.cardStartDescription', '/docs/guides/complete-plugin'],
    ['02', 'landing.cardMidi', 'landing.cardMidiDescription', '/docs/guides/midi'],
    ['03', 'landing.cardWeb', 'landing.cardWebDescription', '/docs/guides/web-ui'],
    ['04', 'landing.cardShip', 'landing.cardShipDescription', '/docs/tooling/packaging']
  ];
</script>

<div class="landing-shell">
  <section class="landing-hero" aria-labelledby="vesty-title">
    <div class="hero-status">
      <span><i aria-hidden="true"></i>{context.t('landing.status')}</span>
      <span>RUST / VST3 / WEBVIEW</span>
      <a href="https://github.com/backrunner/vesty/blob/main/LICENSE-APACHE">APACHE-2.0 ↗</a>
    </div>
    <div class="hero-main">
      <div class="hero-copy">
        <p class="eyebrow"><img src="/brand/vesty-mark.svg" alt="" width="32" height="32" />{page.title} · {context.t('landing.eyebrow')}</p>
        <h1 id="vesty-title">{context.t('landing.headline')}<br /><em>{context.t('landing.headlineAccent')}</em><span class="cursor" aria-hidden="true">_</span></h1>
        <p class="hero-description">{context.t('landing.description')}</p>
        <div class="hero-actions">
          <a class="action-primary" href={resolveLocalizedHref('/docs/quick-start', context)}>{context.t('home.primaryAction')} <span aria-hidden="true">→</span></a>
          <a class="action-secondary" href="https://github.com/backrunner/vesty">{context.t('landing.github')} <span aria-hidden="true">↗</span></a>
        </div>
      </div>
      <div class="terminal" aria-label={context.t('landing.terminalLabel')}>
        <div class="terminal-title"><span aria-hidden="true">⌘</span><span>~/my-plugin</span><span class="terminal-tag">{context.t('landing.preview')}</span></div>
        <div class="terminal-tabs" role="group" aria-label={context.t('landing.templateLabel')}>
          <button type="button" aria-pressed={template === 'gain'} on:click={() => selectTemplate('gain')}>01 / Rust DSP</button>
          <button type="button" aria-pressed={template === 'svelte-ui-param-demo'} on:click={() => selectTemplate('svelte-ui-param-demo')}>02 / + Web UI</button>
        </div>
        <div class="terminal-body">
          <p class="terminal-comment"># {context.t('landing.terminalComment')}</p>
          <div class="terminal-command"><span aria-hidden="true">❯</span><code>{command}</code></div>
          <div class="file-tree" aria-label={context.t('landing.fileTree')}>
            <p><span>my-plugin/</span></p>
            <p>├── <span>src/lib.rs</span><small>// Rust DSP</small></p>
            <p>├── <span>Cargo.toml</span></p>
            <p>├── <span>vesty.toml</span></p>
            <p>├── <span>vesty-parameters.json</span></p>
            {#if template === 'svelte-ui-param-demo'}
              <p>└── <span>ui/</span><small>// Svelte</small></p>
            {:else}
              <p>└── <span>params.specs.json</span></p>
            {/if}
          </div>
          <div class="terminal-signal" aria-hidden="true">
            <span>signal.rs</span>
            <svg viewBox="0 0 420 80" fill="none">
              <path d="M0 40H420" stroke="currentColor" stroke-dasharray="2 6" opacity=".3" />
              <path d="M0 40h45l15-18 21 38 28-46 31 53 33-60 36 67 36-67 33 60 31-53 28 46 21-38 15 18h53" stroke="currentColor" stroke-width="2" />
            </svg>
            <span>f32 / f64</span>
          </div>
        </div>
        <div class="terminal-footer">
          <span aria-live="polite">{context.t(copyState === 'copied' ? 'landing.copied' : copyState === 'failed' ? 'landing.copyFailed' : 'landing.ready')}</span>
          <button type="button" on:click={copyCommand}>{context.t('landing.copy')} <span aria-hidden="true">↗</span></button>
        </div>
      </div>
    </div>
    <div class="hero-bottom">
      <p>{context.t('landing.heroNote')}</p>
      <a class="hero-command" href={resolveLocalizedHref('/docs/concepts/realtime-safety', context)}><span aria-hidden="true">↳</span><code>process() {context.t('landing.boundaryTag')}</code></a>
    </div>
  </section>

  <div class="capability-strip" aria-label={context.t('landing.capabilities')}>
    <span>{context.t('landing.effects')}</span><span>{context.t('landing.instruments')}</span><span>{context.t('landing.automation')}</span><span>React / Vue / Svelte</span>
  </div>

  <section class="signal-section" aria-labelledby="architecture-title">
    <div class="section-heading">
      <p>01 / {context.t('nav.concepts')}</p>
      <h2 id="architecture-title">{context.t('landing.signalTitle')}</h2>
      <span>{context.t('landing.signalDescription')}</span>
    </div>
    <div class="architecture">
      <div class="audio-lane">
        <p class="lane-label">{context.t('landing.audioLane')}</p>
        <ol class="signal-stages">
          <li><span>01</span><strong>{context.t('landing.stageHost')}</strong><small>{context.t('landing.stageHostDescription')}</small></li>
          <li><span>02</span><strong>{context.t('landing.stageAdapter')}</strong><small>{context.t('landing.stageAdapterDescription')}</small></li>
          <li><span>03</span><strong>{context.t('landing.stageKernel')}</strong><small>{context.t('landing.stageKernelDescription')}</small></li>
        </ol>
      </div>
      <div class="control-lane">
        <p class="lane-label">{context.t('landing.controlLane')}</p>
        <p><strong>{context.t('landing.stageEditor')}</strong><span>↔ JSBridge ↔</span><strong>{context.t('landing.controller')}</strong></p>
        <small>{context.t('landing.boundaryNote')}</small>
      </div>
    </div>
    <a class="text-link" href={resolveLocalizedHref('/docs/concepts/architecture', context)}>{context.t('home.secondaryAction')} <span aria-hidden="true">↗</span></a>
  </section>

  <section class="explore-section" aria-labelledby="guides-title">
    <div class="section-heading">
      <p>02 / {context.t('nav.guides')}</p>
      <h2 id="guides-title">{context.t('landing.explore')}</h2>
      <span>{context.t('landing.exploreDescription')}</span>
    </div>
    <div class="guide-grid">
      {#each guides as guide}
        <a href={resolveLocalizedHref(guide[3], context)}>
          <span>{guide[0]}</span>
          <div><strong>{context.t(guide[1])}</strong><small>{context.t(guide[2])}</small></div>
          <i aria-hidden="true">↗</i>
        </a>
      {/each}
    </div>
  </section>

  <section class="contract-section" aria-labelledby="contract-title">
    <div><p>03 / {context.t('landing.contract')}</p><h2 id="contract-title">{context.t('landing.contractTitle')}</h2></div>
    <div><p>{context.t('landing.contractDescription')}</p><a href={resolveLocalizedHref('/docs/concepts/realtime-safety', context)}>{context.t('landing.contractAction')} <span aria-hidden="true">→</span></a></div>
  </section>
  <aside class="alpha-note"><span>α</span><p>{context.t('landing.alphaNote')} <a href={resolveLocalizedHref('/docs/tooling/release-evidence', context)}>{context.t('landing.releaseEvidence')} ↗</a></p></aside>
</div>
